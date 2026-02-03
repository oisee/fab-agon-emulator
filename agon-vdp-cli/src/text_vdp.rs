//! Text-only VDP implementation.
//!
//! Handles VDU commands and outputs text to stdout.
//! Extracted from agon-cli-emulator's fake VDP logic.

use crate::logger::Logger;
use std::collections::VecDeque;
use std::io::Write;

/// Text VDP state
pub struct TextVdp {
    /// Bytes to send back to the eZ80
    tx_queue: VecDeque<u8>,
    /// Whether we're in VDP terminal mode
    terminal_mode: bool,
    /// Partial VDU command being assembled
    pending_cmd: Vec<u8>,
    /// Expected bytes for current command (0 = no command in progress)
    pending_bytes: usize,
    /// Logger for debug output
    logger: Logger,
}

impl TextVdp {
    pub fn new(logger: Logger) -> Self {
        eprintln!("Tom's Fake VDP Version 1.03 (socket)");
        logger.verbose(&format!("[VDP] Debug verbosity: {:?}", logger.verbosity()));
        TextVdp {
            tx_queue: VecDeque::new(),
            terminal_mode: false,
            pending_cmd: Vec::new(),
            pending_bytes: 0,
            logger,
        }
    }

    /// Check if in terminal mode
    pub fn is_terminal_mode(&self) -> bool {
        self.terminal_mode
    }

    /// Format bytes as hex string for debug output
    fn fmt_hex(bytes: &[u8]) -> String {
        bytes
            .iter()
            .map(|b| format!("{:02X}", b))
            .collect::<Vec<_>>()
            .join(" ")
    }

    /// Process a byte from the eZ80
    pub fn process_byte(&mut self, byte: u8) {
        self.logger.trace_uart(&format!("[VDP] <- UART byte: {:02X}", byte));

        // If we're collecting bytes for a command
        if self.pending_bytes > 0 {
            self.pending_cmd.push(byte);
            self.pending_bytes -= 1;
            if self.pending_bytes == 0 {
                self.handle_pending_command();
            }
            return;
        }

        match byte {
            // Ignored bytes
            0 => {
                self.logger.trace("[VDP] VDU 0x00 (ignored - init byte)");
            }
            1 => {
                self.logger.trace("[VDP] VDU 0x01 (ignored)");
            }
            7 => {
                self.logger.trace("[VDP] VDU 0x07 (bell - ignored)");
            }
            9 => {
                self.logger.trace("[VDP] VDU 0x09 (cursor right - ignored)");
            }
            // Newline
            0x0a => {
                self.logger.trace("[VDP] VDU 0x0A (newline)");
                println!();
            }
            // Carriage return
            0x0d => {
                self.logger.trace("[VDP] VDU 0x0D (carriage return)");
            }
            // Color - expect 1 more byte
            0x11 => {
                self.logger.trace("[VDP] VDU 0x11 (color) - waiting for 1 byte");
                self.pending_bytes = 1;
                self.pending_cmd.clear();
                self.pending_cmd.push(byte);
            }
            // Backspace or printable character
            v if v == 8 || (v >= 0x20 && v != 0x7f) => {
                if v == 8 {
                    self.logger.trace("[VDP] VDU 0x08 (backspace)");
                } else {
                    self.logger.trace(&format!("[VDP] VDU 0x{:02X} char '{}'", v, char::from_u32(v as u32).unwrap_or('?')));
                }
                print!("{}", char::from_u32(byte as u32).unwrap());
                std::io::stdout().flush().unwrap();
            }
            // VDP system control
            0x17 => {
                self.logger.trace("[VDP] VDU 0x17 (system control) - waiting for subcommand");
                // Start collecting VDU 0x17 command
                self.pending_bytes = 1; // First byte is subcommand
                self.pending_cmd.clear();
                self.pending_cmd.push(byte);
            }
            // Home cursor
            0x1e => {
                self.logger.trace("[VDP] VDU 0x1E (home cursor - ignored)");
            }
            // Unknown
            _ => {
                self.logger.info(&format!("[VDP] Unknown VDU byte: 0x{:02X}", byte));
            }
        }
    }

    /// Handle a fully assembled pending command
    fn handle_pending_command(&mut self) {
        if self.pending_cmd.is_empty() {
            return;
        }

        match self.pending_cmd[0] {
            // Color command - just ignore the color byte
            0x11 => {
                self.logger.trace(&format!("[VDP] VDU 0x11 color={} (ignored)", self.pending_cmd.get(1).unwrap_or(&0)));
            }
            // VDP system control
            0x17 => {
                if self.pending_cmd.len() < 2 {
                    return;
                }
                match self.pending_cmd[1] {
                    // Video
                    0 => {
                        if self.pending_cmd.len() < 3 {
                            // Need more bytes - wait for next call
                            self.pending_bytes = 1;
                            return;
                        }
                        self.handle_vdu_17_0();
                    }
                    v => {
                        self.logger.info(&format!("[VDP] Unknown VDU 0x17, 0x{:02X}", v));
                    }
                }
            }
            _ => {}
        }
    }

    /// Handle VDU 0x17, 0 (video) commands
    fn handle_vdu_17_0(&mut self) {
        if self.pending_cmd.len() < 3 {
            return;
        }

        match self.pending_cmd[2] {
            // General poll - need 1 more byte
            0x80 => {
                if self.pending_cmd.len() < 4 {
                    self.pending_bytes = 1;
                    return;
                }
                let resp = self.pending_cmd[3];
                self.logger.trace(&format!("[VDP] VDU 0x17,0,0x80 (poll) echo={:02X} -> responding", resp));
                self.send_bytes(&[0x80, 1, resp]);
            }
            // Video mode info
            0x86 => {
                let w: u16 = 640;
                let h: u16 = 400;
                self.logger.trace(&format!("[VDP] VDU 0x17,0,0x86 (mode info) -> {}x{} 80x25", w, h));
                self.send_bytes(&[
                    0x86,
                    7,
                    (w & 0xff) as u8,
                    ((w >> 8) & 0xff) as u8,
                    (h & 0xff) as u8,
                    ((h >> 8) & 0xff) as u8,
                    80,
                    25,
                    1,
                ]);
            }
            // Read RTC - need 1 more byte for mode
            0x87 => {
                if self.pending_cmd.len() < 4 {
                    self.pending_bytes = 1;
                    return;
                }
                let mode = self.pending_cmd[3];
                if mode == 0 {
                    self.logger.trace("[VDP] VDU 0x17,0,0x87 (RTC read) mode=0 -> zeros");
                    self.send_bytes(&[0x87, 6, 0, 0, 0, 0, 0, 0]);
                } else {
                    self.logger.info(&format!("[VDP] Unknown VDU 0x17,0,0x87 mode=0x{:02X}", mode));
                }
            }
            // Enter VDP terminal mode
            0xff => {
                self.logger.info("[VDP] VDU 0x17,0,0xFF -> entering terminal mode");
                self.terminal_mode = true;
            }
            v => {
                self.logger.info(&format!("[VDP] Unknown VDU 0x17,0,0x{:02X} (cmd: {})", v, Self::fmt_hex(&self.pending_cmd)));
            }
        }
    }

    /// Queue bytes to send to the eZ80
    fn send_bytes(&mut self, bytes: &[u8]) {
        self.logger.trace(&format!("[VDP] -> UART response: {}", Self::fmt_hex(bytes)));
        for b in bytes {
            self.tx_queue.push_back(*b);
        }
    }

    /// Get pending bytes to send to eZ80
    pub fn get_tx_bytes(&mut self) -> Vec<u8> {
        self.tx_queue.drain(..).collect()
    }

    /// Create a keyboard event packet
    fn make_key_packet(ascii: u8, down: bool) -> Vec<u8> {
        // cmd, len, keycode, modifiers, vkey, keydown
        vec![0x81, 4, ascii, 0, 0, if down { 1 } else { 0 }]
    }

    /// Generate key events for a line of text (for sending with delays)
    /// Returns a vector of key packets, each should be sent with a delay
    pub fn get_key_events_for_line(&mut self, line: &str) -> Vec<Vec<u8>> {
        self.logger.verbose(&format!("[VDP] Generating key events for: {:?}", line));

        if self.terminal_mode {
            // In terminal mode, send raw bytes (no key events)
            self.logger.trace(&format!("[VDP] -> terminal mode raw: {} bytes", line.len() + 1));
            for ch in line.bytes() {
                self.tx_queue.push_back(ch);
            }
            self.tx_queue.push_back(10); // newline
            vec![] // No key events, data is in tx_queue
        } else {
            // In normal mode, generate keyboard events
            let mut events = Vec::new();
            for ch in line.bytes() {
                let key_char = if ch >= 0x20 && ch < 0x7f {
                    format!("'{}'", ch as char)
                } else {
                    format!("0x{:02X}", ch)
                };
                self.logger.trace(&format!("[VDP] -> KEY {} down", key_char));
                events.push(Self::make_key_packet(ch, true));
                self.logger.trace(&format!("[VDP] -> KEY {} up", key_char));
                events.push(Self::make_key_packet(ch, false));
            }
            // Add Enter key
            self.logger.trace("[VDP] -> KEY 0x0D down");
            events.push(Self::make_key_packet(b'\r', true));
            self.logger.trace("[VDP] -> KEY 0x0D up");
            events.push(Self::make_key_packet(b'\r', false));
            events
        }
    }
}
