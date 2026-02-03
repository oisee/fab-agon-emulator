mod logger;
mod parse_args;
mod socket_link;

use agon_ez80_emulator::{
    debugger::{DebugCmd, DebugResp, DebuggerConnection, PauseReason, Trigger},
    gpio, AgonMachine, AgonMachineConfig, GpioVgaFrame, RamInit,
};
use agon_protocol::{Message, ProtocolError, SocketAddr, SocketConnection, PROTOCOL_VERSION};
use logger::Logger;
use parse_args::{parse_args, Verbosity};
use socket_link::{DummySerialLink, SocketState};

use std::sync::atomic::{AtomicBool, AtomicI32, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::{Duration, Instant};

const PREFIX: Option<&'static str> = option_env!("PREFIX");

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

    // Determine socket address
    let addr = if let Some(tcp) = &args.tcp_addr {
        SocketAddr::tcp(tcp.clone())
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
            eprintln!("Unix sockets not supported on this platform, use --tcp");
            std::process::exit(1);
        }
    };

    // Connect to VDP
    logger.verbose(&format!("[PROTO] Connecting to VDP at {}...", addr));
    if logger.verbosity() < Verbosity::Verbose {
        eprintln!("Connecting to VDP at {}...", addr);
    }
    let mut conn = match SocketConnection::connect(&addr) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to connect to VDP: {}", e);
            eprintln!("Make sure the VDP server is running (e.g., agon-vdp-cli)");
            std::process::exit(1);
        }
    };
    logger.verbose("[PROTO] Connected!");
    if logger.verbosity() < Verbosity::Verbose {
        eprintln!("Connected!");
    }

    // Perform handshake
    if let Err(e) = perform_handshake(&mut conn, &logger) {
        eprintln!("Handshake failed: {}", e);
        std::process::exit(1);
    }
    eprintln!("Handshake complete, starting emulation...");

    // Run the emulator
    if let Err(e) = run_emulator(conn, args, logger) {
        eprintln!("Emulator error: {}", e);
        std::process::exit(1);
    }
}

fn perform_handshake(conn: &mut SocketConnection, logger: &Logger) -> Result<(), ProtocolError> {
    // Send HELLO
    logger.verbose(&format!("[PROTO] -> HELLO version={}, flags=0", PROTOCOL_VERSION));
    conn.send(&Message::Hello {
        version: PROTOCOL_VERSION,
        flags: 0,
    })?;

    // Wait for HELLO_ACK
    let msg = conn.recv()?;
    match msg {
        Message::HelloAck {
            version,
            capabilities,
        } => {
            logger.verbose(&format!("[PROTO] <- HELLO_ACK version={}, caps={}", version, capabilities));
            if logger.verbosity() < Verbosity::Verbose {
                eprintln!(
                    "VDP version {}, capabilities: {}",
                    version,
                    if capabilities.is_empty() {
                        "(none)"
                    } else {
                        &capabilities
                    }
                );
            }
            Ok(())
        }
        _ => Err(ProtocolError::InvalidFormat(
            "Expected HELLO_ACK".to_string(),
        )),
    }
}

fn run_emulator(conn: SocketConnection, args: parse_args::AppArgs, logger: Logger) -> Result<(), ProtocolError> {
    // Shared state
    let socket_state = SocketState::new();
    let soft_reset = Arc::new(AtomicBool::new(false));
    let emulator_shutdown = Arc::new(AtomicBool::new(false));
    let exit_status = Arc::new(AtomicI32::new(0));
    let gpios = Arc::new(gpio::GpioSet::new());
    let ez80_paused = Arc::new(AtomicBool::new(false));

    let (tx_gpio_vga_frame, rx_gpio_vga_frame) = mpsc::channel::<GpioVgaFrame>();

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

    // Start CPU thread
    let gpios_cpu = gpios.clone();
    let emulator_shutdown_cpu = emulator_shutdown.clone();
    let exit_status_cpu = exit_status.clone();
    let ez80_paused_cpu = ez80_paused.clone();
    let uart0_link = socket_state.create_serial_link();

    let _cpu_thread = std::thread::spawn(move || {
        let mut machine = AgonMachine::new(AgonMachineConfig {
            ram_init: if args.zero {
                RamInit::Zero
            } else {
                RamInit::Random
            },
            uart0_link: Box::new(uart0_link),
            uart1_link: Box::new(DummySerialLink),
            soft_reset,
            exit_status: exit_status_cpu,
            paused: ez80_paused_cpu,
            emulator_shutdown: emulator_shutdown_cpu,
            gpios: gpios_cpu,
            tx_gpio_vga_frame,
            interrupt_precision: 16,
            clockspeed_hz: if args.unlimited_cpu {
                1_000_000_000
            } else {
                18_432_000
            },
            mos_bin: args.mos_bin.unwrap_or(default_firmware),
        });

        if let Some(f) = args.sdcard_img {
            match std::fs::File::options().read(true).write(true).open(&f) {
                Ok(file) => machine.set_sdcard_image(Some(file)),
                Err(e) => {
                    eprintln!("Could not open sdcard image '{}': {:?}", f, e);
                    std::process::exit(1);
                }
            }
        } else {
            machine.set_sdcard_directory(match args.sdcard {
                Some(dir) => std::path::PathBuf::from(dir),
                None => std::env::current_dir().unwrap(),
            });
        }

        machine.start(debugger_con);
    });

    // Ignore GPIO VGA frames (only relevant for GPIO video mode)
    std::thread::spawn(move || loop {
        std::thread::sleep(Duration::from_millis(50));
        match rx_gpio_vga_frame.recv() {
            Ok(_) => {}
            Err(_) => break,
        }
    });

    // Split connection for bidirectional communication
    let (mut reader, mut writer) = conn.split();

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
                    emulator_shutdown.store(true, Ordering::Relaxed);
                    break;
                }
                other => {
                    logger.trace(&format!("[PROTO] <- {:?} (unexpected)", other));
                }
            }
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

    let status = exit_status.load(Ordering::Relaxed);
    if status != 0 {
        std::process::exit(status);
    }

    Ok(())
}
