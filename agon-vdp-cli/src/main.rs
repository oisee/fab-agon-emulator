mod logger;
mod parse_args;
mod text_vdp;

use agon_protocol::{Message, ProtocolError, SocketAddr, SocketConnection, SocketListener, PROTOCOL_VERSION};
use logger::Logger;
use parse_args::{parse_args, Verbosity};
use text_vdp::TextVdp;

use std::io::{self, BufRead};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

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
    let addr = if let Some(port) = args.tcp_port {
        SocketAddr::tcp(format!("0.0.0.0:{}", port))
    } else {
        let path = args
            .socket_path
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

    // Bind listener
    let listener = match SocketListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("Failed to bind to {}: {}", addr, e);
            std::process::exit(1);
        }
    };

    eprintln!("Listening on {}", addr);
    eprintln!("Waiting for eZ80 to connect...");

    // Main server loop - accept connections one at a time
    loop {
        match listener.accept() {
            Ok(conn) => {
                logger.verbose("[PROTO] Connection accepted");
                if logger.verbosity() < Verbosity::Verbose {
                    eprintln!("Connection accepted");
                }
                if let Err(e) = handle_connection(conn, &logger) {
                    eprintln!("Connection error: {}", e);
                }
                eprintln!("Connection closed, waiting for new connection...");
            }
            Err(e) => {
                eprintln!("Accept error: {}", e);
                std::thread::sleep(Duration::from_millis(100));
            }
        }
    }
}

/// Format bytes as hex string for debug output
fn fmt_hex(bytes: &[u8]) -> String {
    bytes
        .iter()
        .map(|b| format!("{:02X}", b))
        .collect::<Vec<_>>()
        .join(" ")
}

fn handle_connection(conn: SocketConnection, logger: &Logger) -> Result<(), ProtocolError> {
    let shutdown = Arc::new(AtomicBool::new(false));
    let shutdown_clone = shutdown.clone();

    // Set up stdin reader thread
    let (tx_stdin, rx_stdin): (Sender<String>, Receiver<String>) = mpsc::channel();
    let _stdin_thread = std::thread::spawn(move || {
        let stdin = io::stdin();
        for line in stdin.lock().lines() {
            match line {
                Ok(l) => {
                    if tx_stdin.send(l).is_err() {
                        break;
                    }
                }
                Err(_) => break,
            }
        }
        // Signal EOF
        shutdown_clone.store(true, Ordering::Relaxed);
    });

    // Split connection for bidirectional communication
    let (mut reader, mut writer) = conn.split();

    // Wait for HELLO from eZ80
    logger.verbose("[PROTO] Waiting for HELLO...");
    if logger.verbosity() < Verbosity::Verbose {
        eprintln!("Waiting for HELLO...");
    }
    let msg = reader.recv()?;
    match msg {
        Message::Hello { version, flags } => {
            logger.verbose(&format!("[PROTO] <- HELLO version={}, flags={}", version, flags));
            if logger.verbosity() < Verbosity::Verbose {
                eprintln!("Received HELLO: version={}, flags={}", version, flags);
            }
        }
        _ => {
            return Err(ProtocolError::InvalidFormat(
                "Expected HELLO message".to_string(),
            ));
        }
    }

    // Send HELLO_ACK
    let caps = r#"{"type":"cli","cols":80,"rows":25}"#;
    writer.send(&Message::HelloAck {
        version: PROTOCOL_VERSION,
        capabilities: caps.to_string(),
    })?;
    logger.verbose(&format!("[PROTO] -> HELLO_ACK version={}, caps={}", PROTOCOL_VERSION, caps));
    if logger.verbosity() < Verbosity::Verbose {
        eprintln!("Sent HELLO_ACK");
    }

    // Create text VDP
    let mut vdp = TextVdp::new(logger.clone());

    // Set up reader thread for incoming messages
    let (tx_from_ez80, rx_from_ez80): (Sender<Message>, Receiver<Message>) = mpsc::channel();
    let shutdown_reader = shutdown.clone();
    let _reader_thread = std::thread::spawn(move || {
        loop {
            if shutdown_reader.load(Ordering::Relaxed) {
                break;
            }
            match reader.recv() {
                Ok(msg) => {
                    if tx_from_ez80.send(msg).is_err() {
                        break;
                    }
                }
                Err(ProtocolError::ConnectionClosed) => break,
                Err(e) => {
                    eprintln!("Reader error: {}", e);
                    break;
                }
            }
        }
    });

    // Main loop
    let mut last_vsync = Instant::now();
    let mut last_key_event = Instant::now();
    let vsync_interval = Duration::from_micros(16666); // ~60Hz
    let key_event_interval = Duration::from_millis(10); // 10ms between key events (like original)
    let mut vsync_count: u64 = 0;
    let mut pending_key_events: Vec<Vec<u8>> = Vec::new();

    while !shutdown.load(Ordering::Relaxed) {
        // Process messages from eZ80
        while let Ok(msg) = rx_from_ez80.try_recv() {
            match msg {
                Message::UartData(data) => {
                    logger.trace(&format!("[PROTO] <- UART_DATA ({} bytes): {}", data.len(), fmt_hex(&data)));
                    for byte in data {
                        vdp.process_byte(byte);
                    }
                }
                Message::Shutdown => {
                    logger.verbose("[PROTO] <- SHUTDOWN");
                    if logger.verbosity() < Verbosity::Verbose {
                        eprintln!("Received SHUTDOWN");
                    }
                    return Ok(());
                }
                other => {
                    logger.trace(&format!("[PROTO] <- {:?} (unexpected)", other));
                }
            }
        }

        // Send any pending VDP responses
        let tx_bytes = vdp.get_tx_bytes();
        if !tx_bytes.is_empty() {
            logger.trace(&format!("[PROTO] -> UART_DATA ({} bytes): {}", tx_bytes.len(), fmt_hex(&tx_bytes)));
            writer.send(&Message::UartData(tx_bytes))?;
        }

        // Send VSYNC at ~60Hz
        if last_vsync.elapsed() >= vsync_interval {
            vsync_count += 1;
            if vsync_count % 60 == 0 {
                logger.trace(&format!("[PROTO] -> VSYNC #{} (~{} seconds)", vsync_count, vsync_count / 60));
            }
            writer.send(&Message::Vsync)?;
            last_vsync = last_vsync
                .checked_add(vsync_interval)
                .unwrap_or_else(Instant::now);
        }

        // Process stdin input - queue key events
        if pending_key_events.is_empty() {
            if let Ok(line) = rx_stdin.try_recv() {
                // Get individual key event packets with delays
                pending_key_events = vdp.get_key_events_for_line(&line);

                // Also send any immediate TX bytes (terminal mode raw data)
                let tx_bytes = vdp.get_tx_bytes();
                if !tx_bytes.is_empty() {
                    logger.trace(&format!("[PROTO] -> UART_DATA ({} bytes, terminal): {}", tx_bytes.len(), fmt_hex(&tx_bytes)));
                    writer.send(&Message::UartData(tx_bytes))?;
                }
            }
        }

        // Send pending key events one at a time with delays
        if !pending_key_events.is_empty() && last_key_event.elapsed() >= key_event_interval {
            let key_packet = pending_key_events.remove(0);
            logger.trace(&format!("[PROTO] -> UART_DATA ({} bytes, key): {}", key_packet.len(), fmt_hex(&key_packet)));
            writer.send(&Message::UartData(key_packet))?;
            last_key_event = Instant::now();
        }

        // Small sleep to avoid busy-waiting
        std::thread::sleep(Duration::from_millis(1));
    }

    // Send shutdown
    logger.verbose("[PROTO] -> SHUTDOWN");
    let _ = writer.send(&Message::Shutdown);
    Ok(())
}
