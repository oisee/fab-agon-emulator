/// DZRP TCP Server implementation

use crate::protocol::*;
use crate::translator::*;
use agon_ez80_emulator::debugger::{DebugCmd, DebugResp, PauseReason};
use std::collections::HashMap;
use std::io::{Read, Write};
use std::net::{TcpListener, TcpStream};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;

/// DZRP Server that bridges DeZog IDE to the emulator's debugger
pub struct DzrpServer {
    tx: Sender<DebugCmd>,
    rx: Receiver<DebugResp>,
    shutdown: Arc<AtomicBool>,
    port: u16,
    breakpoint_ids: HashMap<u32, u16>, // address -> DZRP breakpoint ID
    next_bp_id: u16,
    last_pc: u32,
}

impl DzrpServer {
    pub fn new(
        tx: Sender<DebugCmd>,
        rx: Receiver<DebugResp>,
        shutdown: Arc<AtomicBool>,
        port: u16,
    ) -> Self {
        DzrpServer {
            tx,
            rx,
            shutdown,
            port,
            breakpoint_ids: HashMap::new(),
            next_bp_id: 1,
            last_pc: 0,
        }
    }

    /// Run the server main loop
    pub fn run(&mut self) {
        let addr = format!("127.0.0.1:{}", self.port);
        let listener = match TcpListener::bind(&addr) {
            Ok(l) => l,
            Err(e) => {
                eprintln!("DZRP: Failed to bind to {}: {}", addr, e);
                return;
            }
        };

        // Set non-blocking so we can check shutdown flag
        listener
            .set_nonblocking(true)
            .expect("Cannot set non-blocking");

        eprintln!("DZRP: Listening on {} (DeZog remote debugger)", addr);

        while !self.shutdown.load(Ordering::Relaxed) {
            match listener.accept() {
                Ok((stream, client_addr)) => {
                    eprintln!("DZRP: Connection from {}", client_addr);
                    self.handle_connection(stream);
                    eprintln!("DZRP: Connection closed");
                }
                Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    // No connection pending, sleep briefly
                    std::thread::sleep(Duration::from_millis(100));
                }
                Err(e) => {
                    eprintln!("DZRP: Accept error: {}", e);
                }
            }
        }

        eprintln!("DZRP: Server shutdown");
    }

    /// Handle a single client connection
    fn handle_connection(&mut self, mut stream: TcpStream) {
        // Set read timeout for non-blocking checks
        stream
            .set_read_timeout(Some(Duration::from_millis(50)))
            .ok();
        stream
            .set_write_timeout(Some(Duration::from_millis(1000)))
            .ok();

        let mut buffer = vec![0u8; 65536];
        let mut pending_data = Vec::new();

        while !self.shutdown.load(Ordering::Relaxed) {
            // Check for responses from the debugger (async notifications)
            self.check_debug_responses(&mut stream);

            // Try to read data from client
            match stream.read(&mut buffer) {
                Ok(0) => {
                    // Connection closed
                    break;
                }
                Ok(n) => {
                    pending_data.extend_from_slice(&buffer[..n]);

                    // Process complete messages
                    while let Some((msg, consumed)) = self.try_parse_message(&pending_data) {
                        pending_data.drain(..consumed);

                        if let Some(response) = self.handle_message(&msg) {
                            if stream.write_all(&response).is_err() {
                                return;
                            }
                        }

                        // Check if this was a close command
                        if msg.cmd_id == CMD_CLOSE {
                            return;
                        }
                    }
                }
                Err(ref e)
                    if e.kind() == std::io::ErrorKind::WouldBlock
                        || e.kind() == std::io::ErrorKind::TimedOut =>
                {
                    // No data available, continue
                }
                Err(_) => {
                    // Connection error
                    break;
                }
            }
        }
    }

    /// Try to parse a complete DZRP message from the buffer
    fn try_parse_message(&self, data: &[u8]) -> Option<(DzrpMessage, usize)> {
        if data.len() < 4 {
            return None;
        }

        // Read length (4 bytes LE)
        let len = read_u32_le(data, 0) as usize;
        let total_len = 4 + len;

        if data.len() < total_len {
            return None;
        }

        // Parse message content (after length prefix)
        let msg = DzrpMessage::parse(&data[4..total_len])?;
        Some((msg, total_len))
    }

    /// Check for async responses from the debugger
    fn check_debug_responses(&mut self, stream: &mut TcpStream) {
        loop {
            match self.rx.try_recv() {
                Ok(resp) => {
                    // Handle state responses to track PC
                    if let DebugResp::State { registers, .. } = &resp {
                        self.last_pc = registers.pc;
                    }

                    // Send notification for pause events
                    if let DebugResp::Paused(reason) = &resp {
                        let payload = pause_to_notification_payload(reason, self.last_pc);
                        let notification = create_notification(NTF_PAUSE, &payload);
                        let _ = stream.write_all(&notification);
                    }
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => break,
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    self.shutdown.store(true, Ordering::Relaxed);
                    break;
                }
            }
        }
    }

    /// Handle a DZRP message and return the response
    fn handle_message(&mut self, msg: &DzrpMessage) -> Option<Vec<u8>> {
        match msg.cmd_id {
            CMD_INIT => {
                let payload = create_init_response();
                Some(msg.response(payload))
            }
            CMD_CLOSE => {
                // Send empty success response
                Some(msg.response(vec![]))
            }
            CMD_LOOPBACK => {
                // Echo back the payload
                Some(msg.response(msg.payload.clone()))
            }
            CMD_ADD_BREAKPOINT => {
                // Extract breakpoint ID from payload
                let bp_id = if msg.payload.len() >= 2 {
                    read_u16_le(&msg.payload, 0)
                } else {
                    self.next_bp_id
                };
                self.next_bp_id = self.next_bp_id.wrapping_add(1);

                // Get address from payload
                let address = if msg.payload.len() >= 7 {
                    read_u24_le(&msg.payload, 4)
                } else {
                    return Some(msg.response(vec![1])); // Error
                };

                // Store mapping
                self.breakpoint_ids.insert(address, bp_id);

                // Send to debugger
                if let Some(cmds) = dzrp_to_debug_cmd(msg) {
                    for cmd in cmds {
                        self.tx.send(cmd).ok();
                    }
                    // Wait for response
                    self.wait_for_pong();
                }

                // Return success with breakpoint ID
                let mut response = vec![0u8]; // Success
                write_u16_le(&mut response, bp_id);
                Some(msg.response(response))
            }
            CMD_REMOVE_BREAKPOINT => {
                if let Some(cmds) = dzrp_to_debug_cmd(msg) {
                    for cmd in cmds {
                        self.tx.send(cmd).ok();
                    }
                    self.wait_for_pong();
                }
                Some(msg.response(vec![]))
            }
            CMD_GET_REGISTERS => {
                self.tx.send(DebugCmd::GetRegisters).ok();
                if let Some(resp) = self.wait_for_response() {
                    if let Some(payload) = debug_resp_to_dzrp(&resp) {
                        return Some(msg.response(payload));
                    }
                }
                Some(msg.response(vec![]))
            }
            CMD_SET_REGISTER => {
                if let Some(cmds) = dzrp_to_debug_cmd(msg) {
                    for cmd in cmds {
                        self.tx.send(cmd).ok();
                    }
                    self.wait_for_pong();
                }
                Some(msg.response(vec![]))
            }
            CMD_READ_MEM => {
                if let Some(cmds) = dzrp_to_debug_cmd(msg) {
                    for cmd in cmds {
                        self.tx.send(cmd).ok();
                    }
                    if let Some(resp) = self.wait_for_response() {
                        if let Some(payload) = debug_resp_to_dzrp(&resp) {
                            return Some(msg.response(payload));
                        }
                    }
                }
                Some(msg.response(vec![]))
            }
            CMD_WRITE_MEM => {
                if let Some(cmds) = dzrp_to_debug_cmd(msg) {
                    for cmd in cmds {
                        self.tx.send(cmd).ok();
                    }
                    self.wait_for_pong();
                }
                Some(msg.response(vec![]))
            }
            CMD_CONTINUE => {
                self.tx.send(DebugCmd::Continue).ok();
                self.wait_for_response();
                Some(msg.response(vec![]))
            }
            CMD_PAUSE => {
                self.tx
                    .send(DebugCmd::Pause(PauseReason::DebuggerRequested))
                    .ok();
                self.tx.send(DebugCmd::GetState).ok();
                // Don't wait - notification will be sent async
                Some(msg.response(vec![]))
            }
            CMD_STEP_INTO => {
                self.tx.send(DebugCmd::Step).ok();
                if let Some(DebugResp::State { registers, .. }) = self.wait_for_response() {
                    self.last_pc = registers.pc;
                }
                Some(msg.response(vec![]))
            }
            CMD_STEP_OVER => {
                self.tx.send(DebugCmd::StepOver).ok();
                // Step over may resume, wait for response
                if let Some(resp) = self.wait_for_response() {
                    if let DebugResp::State { registers, .. } = resp {
                        self.last_pc = registers.pc;
                    }
                }
                Some(msg.response(vec![]))
            }
            _ => {
                // Unknown command - return empty response
                eprintln!("DZRP: Unknown command 0x{:02x}", msg.cmd_id);
                Some(msg.response(vec![]))
            }
        }
    }

    /// Wait for a response from the debugger
    fn wait_for_response(&mut self) -> Option<DebugResp> {
        let timeout = Duration::from_secs(5);
        let start = std::time::Instant::now();

        loop {
            match self.rx.try_recv() {
                Ok(resp) => {
                    // Track PC from state responses
                    if let DebugResp::State { registers, .. } = &resp {
                        self.last_pc = registers.pc;
                    }
                    return Some(resp);
                }
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    if start.elapsed() > timeout {
                        return None;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => {
                    return None;
                }
            }
        }
    }

    /// Wait for a Pong response (acknowledgment)
    fn wait_for_pong(&mut self) {
        let timeout = Duration::from_secs(1);
        let start = std::time::Instant::now();

        loop {
            match self.rx.try_recv() {
                Ok(DebugResp::Pong) => return,
                Ok(DebugResp::State { registers, .. }) => {
                    self.last_pc = registers.pc;
                }
                Ok(_) => {}
                Err(std::sync::mpsc::TryRecvError::Empty) => {
                    if start.elapsed() > timeout {
                        return;
                    }
                    std::thread::sleep(Duration::from_millis(1));
                }
                Err(std::sync::mpsc::TryRecvError::Disconnected) => return,
            }
        }
    }
}
