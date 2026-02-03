//! SerialLink implementation over socket protocol.

use agon_ez80_emulator::SerialLink;
use std::collections::VecDeque;
use std::sync::{Arc, Mutex};

/// SerialLink implementation that communicates over socket protocol.
///
/// This is used for UART0 (eZ80 <-> VDP communication).
pub struct SocketSerialLink {
    /// Shared send queue - bytes are queued here and sent by the main thread
    tx_queue: Arc<Mutex<VecDeque<u8>>>,
    /// Shared receive queue - bytes received from VDP are placed here
    rx_queue: Arc<Mutex<VecDeque<u8>>>,
    /// Clear-to-send status
    cts: Arc<Mutex<bool>>,
}

impl SocketSerialLink {
    pub fn new(
        tx_queue: Arc<Mutex<VecDeque<u8>>>,
        rx_queue: Arc<Mutex<VecDeque<u8>>>,
        cts: Arc<Mutex<bool>>,
    ) -> Self {
        SocketSerialLink {
            tx_queue,
            rx_queue,
            cts,
        }
    }
}

impl SerialLink for SocketSerialLink {
    fn send(&mut self, byte: u8) {
        if let Ok(mut queue) = self.tx_queue.lock() {
            queue.push_back(byte);
        }
    }

    fn recv(&mut self) -> Option<u8> {
        if let Ok(mut queue) = self.rx_queue.lock() {
            queue.pop_front()
        } else {
            None
        }
    }

    fn read_clear_to_send(&mut self) -> bool {
        if let Ok(cts) = self.cts.lock() {
            *cts
        } else {
            true
        }
    }
}

/// Dummy serial link that does nothing (used for UART1)
pub struct DummySerialLink;

impl SerialLink for DummySerialLink {
    fn send(&mut self, _byte: u8) {}
    fn recv(&mut self) -> Option<u8> {
        None
    }
    fn read_clear_to_send(&mut self) -> bool {
        true
    }
}

/// Shared state for socket communication
pub struct SocketState {
    pub tx_queue: Arc<Mutex<VecDeque<u8>>>,
    pub rx_queue: Arc<Mutex<VecDeque<u8>>>,
    pub cts: Arc<Mutex<bool>>,
}

impl SocketState {
    pub fn new() -> Self {
        SocketState {
            tx_queue: Arc::new(Mutex::new(VecDeque::new())),
            rx_queue: Arc::new(Mutex::new(VecDeque::new())),
            cts: Arc::new(Mutex::new(true)),
        }
    }

    /// Create a SerialLink for this socket state
    pub fn create_serial_link(&self) -> SocketSerialLink {
        SocketSerialLink::new(
            self.tx_queue.clone(),
            self.rx_queue.clone(),
            self.cts.clone(),
        )
    }

    /// Drain pending TX bytes and send them
    pub fn drain_tx(&self) -> Vec<u8> {
        if let Ok(mut queue) = self.tx_queue.lock() {
            queue.drain(..).collect()
        } else {
            vec![]
        }
    }

    /// Queue received bytes from VDP
    pub fn queue_rx(&self, bytes: &[u8]) {
        if let Ok(mut queue) = self.rx_queue.lock() {
            for b in bytes {
                queue.push_back(*b);
            }
        }
    }

    /// Update CTS status
    pub fn set_cts(&self, ready: bool) {
        if let Ok(mut cts) = self.cts.lock() {
            *cts = ready;
        }
    }
}

impl Default for SocketState {
    fn default() -> Self {
        Self::new()
    }
}
