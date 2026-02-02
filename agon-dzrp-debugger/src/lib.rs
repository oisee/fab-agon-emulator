mod protocol;
mod server;
mod translator;

use agon_ez80_emulator::debugger::{DebugCmd, DebugResp};
use std::sync::atomic::AtomicBool;
use std::sync::mpsc::{Receiver, Sender};
use std::sync::Arc;

pub use server::DzrpServer;

/// Default port for DZRP connections
pub const DEFAULT_PORT: u16 = 11000;

/// Start the DZRP debugger server
pub fn start(
    tx: Sender<DebugCmd>,
    rx: Receiver<DebugResp>,
    shutdown: Arc<AtomicBool>,
    port: u16,
) {
    let mut server = DzrpServer::new(tx, rx, shutdown, port);
    server.run();
}
