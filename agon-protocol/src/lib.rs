//! # Agon Protocol
//!
//! A language-agnostic protocol for communication between eZ80 and VDP components.
//!
//! ## Wire Format
//!
//! Simple length-prefixed binary messages:
//! ```text
//! [len:u16-LE][type:u8][payload...]
//! ```
//!
//! ## Message Types
//!
//! | Type | Name | Direction | Payload |
//! |------|------|-----------|---------|
//! | 0x01 | UART_DATA | bidirectional | raw bytes (1-1024) |
//! | 0x02 | VSYNC | VDP→eZ80 | empty |
//! | 0x03 | CTS | VDP→eZ80 | u8 (0=busy, 1=ready) |
//! | 0x10 | HELLO | eZ80→VDP | version:u8, flags:u8 |
//! | 0x11 | HELLO_ACK | VDP→eZ80 | version:u8, caps_json |
//! | 0x20 | SHUTDOWN | either | empty |

mod messages;
pub mod socket;

pub use messages::{Message, ProtocolError, PROTOCOL_VERSION};
pub use socket::{SocketAddr, SocketConnection, SocketListener, SocketReader, SocketWriter};
