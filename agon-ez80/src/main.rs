mod logger;
mod parse_args;
mod socket_link;

use agon_ez80_emulator::{
    debugger::{DebugCmd, DebugResp, DebuggerConnection, PauseReason, Trigger},
    gpio, AgonMachine, AgonMachineConfig, GpioVgaFrame, RamInit,
};
use agon_protocol::{Message, ProtocolError, SocketAddr, SocketListener, WebSocketConnection, WebSocketListener, PROTOCOL_VERSION};
use logger::Logger;
use parse_args::{parse_args, Verbosity};
use socket_link::{DummySerialLink, SocketState};

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const PREFIX: Option<&'static str> = option_env!("PREFIX");

/// Listener type for accepting VDP connections
enum Listener {
    Socket(SocketListener),
    WebSocket(WebSocketListener),
}

/// Format bytes as hex string for debug output
fn fmt_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn main() {
    let args = match parse_args() {
        Ok(a) => a,
        Err(e) => {
            eprintln!("Error parsing arguments: {}", e);
            std::process::exit(1);
        }
    };

    // Set up logger
    let logger = match &args.log_file {
        Some(path) => {
            match Logger::file(path, args.verbosity) {
                Ok(l) => {
                    eprintln!("Logging to: {}", path);
                    l
                }
                Err(e) => {
                    eprintln!("Failed to open log file '{}': {}", path, e);
                    std::process::exit(1);
                }
            }
        }
        None => Logger::stderr(args.verbosity),
    };

    // Create listener based on options
    let listener = if let Some(port) = args.websocket_port {
        // WebSocket mode
        match WebSocketListener::bind(port) {
            Ok(l) => {
                eprintln!("Listening for WebSocket connections on ws://0.0.0.0:{}", port);
                Listener::WebSocket(l)
            }
            Err(e) => {
                eprintln!("Failed to bind WebSocket to port {}: {}", port, e);
                std::process::exit(1);
            }
        }
    } else {
        // Socket mode (Unix or TCP)
        let addr = if let Some(port) = args.tcp_port {
            SocketAddr::tcp(format!("0.0.0.0:{}", port))
        } else {
            let path = args
                .socket_path
                .clone()
                .unwrap_or_else(|| agon_protocol::socket::DEFAULT_SOCKET_PATH.to_string());
            #[cfg(unix)]
            {
                SocketAddr::unix(&path)
            }
            #[cfg(not(unix))]
            {
                eprintln!("Unix sockets not supported on this platform, use --tcp or --websocket");
                std::process::exit(1);
            }
        };

        match SocketListener::bind(&addr) {
            Ok(l) => {
                eprintln!("Listening on {}", addr);
                Listener::Socket(l)
            }
            Err(e) => {
                eprintln!("Failed to bind to {}: {}", addr, e);
                std::process::exit(1);
            }
        }
    };

    // Shared state for CPU communication (persists across VDP reconnections)
    let socket_state = SocketState::new();
    let soft_reset = Arc::new(AtomicBool::new(false));
    let emulator_shutdown = Arc::new(AtomicBool::new(false));
    let exit_status = Arc::new(AtomicI32::new(0));
    let gpios = Arc::new(gpio::GpioSet::new());
    let ez80_paused = Arc::new(AtomicBool::new(false));

    // Default firmware path
    let default_firmware = match PREFIX {
        None => std::path::Path::new(".")
            .join("firmware")
            .join("mos_console8.bin"),
        Some(prefix) => std::path::Path::new(prefix)
            .join("share")
            .join("fab-agon-emulator")
            .join("mos_console8.bin"),
    };

    eprintln!("Waiting for VDP to connect...");

    // Track if CPU has been started (only start on first VDP connection)
    let mut cpu_started = false;

    // Helper closure to start CPU on first VDP connection
    let start_cpu = |cpu_started: &mut bool| {
        if *cpu_started {
            return;
        }

        // Set up debugger if requested
        let (tx_cmd_debugger, rx_cmd_debugger): (Sender<DebugCmd>, Receiver<DebugCmd>) =
            mpsc::channel();
        let (tx_resp_debugger, rx_resp_debugger): (Sender<DebugResp>, Receiver<DebugResp>) =
            mpsc::channel();

        let debugger_con = if args.debugger {
            let _ez80_paused = ez80_paused.clone();
            let _emulator_shutdown = emulator_shutdown.clone();
            let _breakpoints = args.breakpoints.clone();
            let _tx_cmd = tx_cmd_debugger.clone();

            std::thread::spawn(move || {
                // Set initial breakpoints
                for bp in _breakpoints {
                    let trigger = Trigger {
                        address: bp,
                        once: false,
                        actions: vec![
                            DebugCmd::Pause(PauseReason::DebuggerBreakpoint),
                            DebugCmd::GetState,
                        ],
                    };
                    let _ = _tx_cmd.send(DebugCmd::AddTrigger(trigger));
                }

                agon_light_emulator_debugger::start(
                    _tx_cmd,
                    rx_resp_debugger,
                    _emulator_shutdown,
                    _ez80_paused.load(Ordering::Relaxed),
                );
            });

            Some(DebuggerConnection {
                tx: tx_resp_debugger,
                rx: rx_cmd_debugger,
            })
        } else {
            None
        };

        let (tx_gpio_vga_frame, rx_gpio_vga_frame) = mpsc::channel::<GpioVgaFrame>();

        // Ignore GPIO VGA frames (only relevant for GPIO video mode)
        std::thread::spawn(move || loop {
            std::thread::sleep(Duration::from_millis(50));
            match rx_gpio_vga_frame.recv() {
                Ok(_) => {}
                Err(_) => break,
            }
        });

        let gpios_cpu = gpios.clone();
        let emulator_shutdown_cpu = emulator_shutdown.clone();
        let exit_status_cpu = exit_status.clone();
        let ez80_paused_cpu = ez80_paused.clone();
        let soft_reset_cpu = soft_reset.clone();
        let uart0_link = socket_state.create_serial_link();
        let mos_bin = args.mos_bin.clone().unwrap_or_else(|| default_firmware.clone());
        let sdcard = args.sdcard.clone();
        let sdcard_img = args.sdcard_img.clone();
        let unlimited_cpu = args.unlimited_cpu;
        let zero = args.zero;

        std::thread::spawn(move || {
            let mut machine = AgonMachine::new(AgonMachineConfig {
                ram_init: if zero {
                    RamInit::Zero
                } else {
                    RamInit::Random
                },
                uart0_link: Box::new(uart0_link),
                uart1_link: Box::new(DummySerialLink),
                soft_reset: soft_reset_cpu,
                exit_status: exit_status_cpu,
                paused: ez80_paused_cpu,
                emulator_shutdown: emulator_shutdown_cpu,
                gpios: gpios_cpu,
                tx_gpio_vga_frame,
                interrupt_precision: 16,
                clockspeed_hz: if unlimited_cpu {
                    1_000_000_000
                } else {
                    18_432_000
                },
                mos_bin,
            });

            if let Some(f) = sdcard_img {
                match std::fs::File::options().read(true).write(true).open(&f) {
                    Ok(file) => machine.set_sdcard_image(Some(file)),
                    Err(e) => {
                        eprintln!("Could not open sdcard image '{}': {:?}", f, e);
                        std::process::exit(1);
                    }
                }
            } else {
                machine.set_sdcard_directory(match sdcard {
                    Some(dir) => std::path::PathBuf::from(dir),
                    None => std::env::current_dir().unwrap(),
                });
            }

            machine.start(debugger_con);
        });

        *cpu_started = true;
        eprintln!("eZ80 CPU started");
    };

    // Main server loop - accept VDP connections (supports reconnection)
    loop {
        let session_result = match &listener {
            Listener::Socket(sock_listener) => {
                match sock_listener.accept() {
                    Ok(conn) => {
                        logger.verbose("[PROTO] VDP connected (socket)");
                        if logger.verbosity() < Verbosity::Verbose {
                            eprintln!("VDP connected");
                        }
                        start_cpu(&mut cpu_started);
                        handle_vdp_session(conn, &socket_state, &gpios, &emulator_shutdown, &logger)
                    }
                    Err(e) => {
                        eprintln!("Accept error: {}", e);
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                }
            }
            Listener::WebSocket(ws_listener) => {
                match ws_listener.accept() {
                    Ok(conn) => {
                        logger.verbose("[PROTO] VDP connected (WebSocket)");
                        if logger.verbosity() < Verbosity::Verbose {
                            eprintln!("WebSocket VDP connected");
                        }
                        start_cpu(&mut cpu_started);
                        handle_vdp_websocket_session(conn, &socket_state, &gpios, &emulator_shutdown, &logger)
                    }
                    Err(e) => {
                        eprintln!("WebSocket accept error: {}", e);
                        std::thread::sleep(Duration::from_millis(100));
                        continue;
                    }
                }
            }
        };

        if let Err(e) = session_result {
            eprintln!("VDP session error: {}", e);
        }
        if emulator_shutdown.load(Ordering::Relaxed) {
            break;
        }
        eprintln!("VDP disconnected, waiting for reconnection...");
    }

    let status = exit_status.load(Ordering::Relaxed);
    if status != 0 {
        std::process::exit(status);
    }
}

fn handle_vdp_session(
    conn: agon_protocol::SocketConnection,
    socket_state: &SocketState,
    gpios: &Arc<gpio::GpioSet>,
    emulator_shutdown: &Arc<AtomicBool>,
    logger: &Logger,
) -> Result<(), ProtocolError> {
    // Split connection for bidirectional communication
    let (mut reader, mut writer) = conn.split();

    // Wait for HELLO from VDP (VDP is the connector, so it sends HELLO)
    logger.verbose("[PROTO] Waiting for HELLO from VDP...");
    let msg = reader.recv()?;
    match msg {
        Message::Hello { version, flags } => {
            logger.verbose(&format!("[PROTO] <- HELLO version={}, flags={}", version, flags));
            if logger.verbosity() < Verbosity::Verbose {
                eprintln!("VDP version {}, flags={}", version, flags);
            }
        }
        _ => {
            return Err(ProtocolError::InvalidFormat(
                "Expected HELLO from VDP".to_string(),
            ));
        }
    }

    // Send HELLO_ACK
    let caps = r#"{"type":"ez80","version":"1.0"}"#;
    writer.send(&Message::HelloAck {
        version: PROTOCOL_VERSION,
        capabilities: caps.to_string(),
    })?;
    logger.verbose(&format!("[PROTO] -> HELLO_ACK version={}, caps={}", PROTOCOL_VERSION, caps));
    if logger.verbosity() < Verbosity::Verbose {
        eprintln!("Handshake complete");
    }

    // Set up reader thread
    let (tx_from_vdp, rx_from_vdp): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let emulator_shutdown_reader = emulator_shutdown.clone();

    std::thread::spawn(move || loop {
        if emulator_shutdown_reader.load(Ordering::Relaxed) {
            break;
        }
        match reader.recv() {
            Ok(msg) => {
                if tx_from_vdp.send(msg).is_err() {
                    break;
                }
            }
            Err(ProtocolError::ConnectionClosed) => break,
            Err(e) => {
                eprintln!("Socket read error: {}", e);
                break;
            }
        }
    });

    // Main communication loop
    let mut last_tx_time = Instant::now();
    let tx_interval = Duration::from_micros(100); // Send at most every 100us
    let mut vsync_count: u64 = 0;

    while !emulator_shutdown.load(Ordering::Relaxed) {
        // Process messages from VDP
        let mut vdp_disconnected = false;
        while let Ok(msg) = rx_from_vdp.try_recv() {
            match msg {
                Message::UartData(data) => {
                    logger.trace(&format!("[PROTO] <- UART_DATA ({} bytes): {}", data.len(), fmt_hex(&data)));
                    socket_state.queue_rx(&data);
                }
                Message::Vsync => {
                    vsync_count += 1;
                    if vsync_count % 60 == 0 {
                        logger.trace(&format!("[PROTO] <- VSYNC #{} (~{} seconds)", vsync_count, vsync_count / 60));
                    }
                    // Signal vsync to eZ80 via GPIO (pin 1 of GPIO port B)
                    gpios.b.set_input_pin(1, true);
                    gpios.b.set_input_pin(1, false);
                }
                Message::Cts(ready) => {
                    logger.trace(&format!("[PROTO] <- CTS ready={}", ready));
                    socket_state.set_cts(ready);
                }
                Message::Shutdown => {
                    logger.verbose("[PROTO] <- SHUTDOWN");
                    if logger.verbosity() < Verbosity::Verbose {
                        eprintln!("VDP requested shutdown");
                    }
                    vdp_disconnected = true;
                    break;
                }
                other => {
                    logger.trace(&format!("[PROTO] <- {:?} (unexpected)", other));
                }
            }
        }

        if vdp_disconnected {
            break;
        }

        // Send pending TX bytes to VDP (batched)
        if last_tx_time.elapsed() >= tx_interval {
            let tx_bytes = socket_state.drain_tx();
            if !tx_bytes.is_empty() {
                logger.trace(&format!("[PROTO] -> UART_DATA ({} bytes): {}", tx_bytes.len(), fmt_hex(&tx_bytes)));
                if let Err(e) = writer.send(&Message::UartData(tx_bytes)) {
                    eprintln!("Socket write error: {}", e);
                    break;
                }
            }
            last_tx_time = Instant::now();
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_micros(100));
    }

    // Send shutdown to VDP
    logger.verbose("[PROTO] -> SHUTDOWN");
    let _ = writer.send(&Message::Shutdown);

    Ok(())
}

fn handle_vdp_websocket_session(
    mut conn: WebSocketConnection,
    socket_state: &SocketState,
    gpios: &Arc<gpio::GpioSet>,
    emulator_shutdown: &Arc<AtomicBool>,
    logger: &Logger,
) -> Result<(), ProtocolError> {
    // Wait for HELLO from VDP (VDP is the connector, so it sends HELLO)
    logger.verbose("[PROTO] Waiting for HELLO from WebSocket VDP...");
    let msg = conn.recv()?;
    match msg {
        Message::Hello { version, flags } => {
            logger.verbose(&format!("[PROTO] <- HELLO version={}, flags={}", version, flags));
            if logger.verbosity() < Verbosity::Verbose {
                eprintln!("WebSocket VDP version {}, flags={}", version, flags);
            }
        }
        _ => {
            return Err(ProtocolError::InvalidFormat(
                "Expected HELLO from VDP".to_string(),
            ));
        }
    }

    // Send HELLO_ACK
    let caps = r#"{"type":"ez80","version":"1.0"}"#;
    conn.send(&Message::HelloAck {
        version: PROTOCOL_VERSION,
        capabilities: caps.to_string(),
    })?;
    logger.verbose(&format!("[PROTO] -> HELLO_ACK version={}, caps={}", PROTOCOL_VERSION, caps));
    if logger.verbosity() < Verbosity::Verbose {
        eprintln!("WebSocket handshake complete");
    }

    // Main communication loop (WebSocket is already message-based, no need for split)
    let mut last_tx_time = Instant::now();
    let tx_interval = Duration::from_micros(100);
    let mut vsync_count: u64 = 0;

    while !emulator_shutdown.load(Ordering::Relaxed) {
        // Try to receive messages from VDP (non-blocking)
        let mut vdp_disconnected = false;
        match conn.try_recv() {
            Ok(Some(msg)) => match msg {
                Message::UartData(data) => {
                    logger.trace(&format!("[PROTO] <- UART_DATA ({} bytes): {}", data.len(), fmt_hex(&data)));
                    socket_state.queue_rx(&data);
                }
                Message::Vsync => {
                    vsync_count += 1;
                    if vsync_count % 60 == 0 {
                        logger.trace(&format!("[PROTO] <- VSYNC #{} (~{} seconds)", vsync_count, vsync_count / 60));
                    }
                    gpios.b.set_input_pin(1, true);
                    gpios.b.set_input_pin(1, false);
                }
                Message::Cts(ready) => {
                    logger.trace(&format!("[PROTO] <- CTS ready={}", ready));
                    socket_state.set_cts(ready);
                }
                Message::Shutdown => {
                    logger.verbose("[PROTO] <- SHUTDOWN");
                    if logger.verbosity() < Verbosity::Verbose {
                        eprintln!("WebSocket VDP requested shutdown");
                    }
                    vdp_disconnected = true;
                }
                other => {
                    logger.trace(&format!("[PROTO] <- {:?} (unexpected)", other));
                }
            },
            Ok(None) => {
                // No message available
            }
            Err(e) => {
                eprintln!("WebSocket read error: {}", e);
                vdp_disconnected = true;
            }
        }

        if vdp_disconnected {
            break;
        }

        // Send pending TX bytes to VDP (batched)
        if last_tx_time.elapsed() >= tx_interval {
            let tx_bytes = socket_state.drain_tx();
            if !tx_bytes.is_empty() {
                logger.trace(&format!("[PROTO] -> UART_DATA ({} bytes): {}", tx_bytes.len(), fmt_hex(&tx_bytes)));
                if let Err(e) = conn.send(&Message::UartData(tx_bytes)) {
                    eprintln!("WebSocket write error: {}", e);
                    break;
                }
            }
            last_tx_time = Instant::now();
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_micros(100));
    }

    // Send shutdown to VDP
    logger.verbose("[PROTO] -> SHUTDOWN");
    let _ = conn.send(&Message::Shutdown);

    Ok(())
}
