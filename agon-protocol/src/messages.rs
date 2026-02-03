//! Message types and encoding/decoding for the eZ80/VDP protocol.

use std::io::{Read, Write};

/// Protocol version number
pub const PROTOCOL_VERSION: u8 = 1;

/// Maximum payload size for UART_DATA messages
pub const MAX_UART_DATA_SIZE: usize = 1024;

/// Message type constants
mod msg_type {
    pub const UART_DATA: u8 = 0x01;
    pub const VSYNC: u8 = 0x02;
    pub const CTS: u8 = 0x03;
    pub const HELLO: u8 = 0x10;
    pub const HELLO_ACK: u8 = 0x11;
    pub const SHUTDOWN: u8 = 0x20;
}

/// Protocol error types
#[derive(Debug)]
pub enum ProtocolError {
    /// I/O error during read/write
    Io(std::io::Error),
    /// Unknown message type received
    UnknownMessageType(u8),
    /// Message payload too large
    PayloadTooLarge(usize),
    /// Invalid message format
    InvalidFormat(String),
    /// Connection closed
    ConnectionClosed,
}

impl std::fmt::Display for ProtocolError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ProtocolError::Io(e) => write!(f, "I/O error: {}", e),
            ProtocolError::UnknownMessageType(t) => write!(f, "Unknown message type: 0x{:02x}", t),
            ProtocolError::PayloadTooLarge(size) => write!(f, "Payload too large: {} bytes", size),
            ProtocolError::InvalidFormat(msg) => write!(f, "Invalid format: {}", msg),
            ProtocolError::ConnectionClosed => write!(f, "Connection closed"),
        }
    }
}

impl std::error::Error for ProtocolError {}

impl From<std::io::Error> for ProtocolError {
    fn from(e: std::io::Error) -> Self {
        if e.kind() == std::io::ErrorKind::UnexpectedEof {
            ProtocolError::ConnectionClosed
        } else {
            ProtocolError::Io(e)
        }
    }
}

/// Messages exchanged between eZ80 and VDP over socket
#[derive(Debug, Clone, PartialEq)]
pub enum Message {
    /// UART data bytes (bidirectional)
    UartData(Vec<u8>),

    /// VSync signal from VDP to eZ80
    Vsync,

    /// Clear-to-send status from VDP to eZ80
    Cts(bool),

    /// Hello message from eZ80 to VDP during connection setup
    Hello {
        version: u8,
        flags: u8,
    },

    /// Hello acknowledgment from VDP to eZ80
    HelloAck {
        version: u8,
        capabilities: String,
    },

    /// Shutdown request (either direction)
    Shutdown,
}

impl Message {
    /// Encode message to wire format
    pub fn encode(&self) -> Vec<u8> {
        let (msg_type, payload) = match self {
            Message::UartData(data) => (msg_type::UART_DATA, data.clone()),
            Message::Vsync => (msg_type::VSYNC, vec![]),
            Message::Cts(ready) => (msg_type::CTS, vec![if *ready { 1 } else { 0 }]),
            Message::Hello { version, flags } => (msg_type::HELLO, vec![*version, *flags]),
            Message::HelloAck {
                version,
                capabilities,
            } => {
                let mut p = vec![*version];
                p.extend(capabilities.as_bytes());
                (msg_type::HELLO_ACK, p)
            }
            Message::Shutdown => (msg_type::SHUTDOWN, vec![]),
        };

        // Format: [len:u16-LE][type:u8][payload...]
        // len includes the type byte
        let len = (1 + payload.len()) as u16;
        let mut result = Vec::with_capacity(2 + len as usize);
        result.extend(&len.to_le_bytes());
        result.push(msg_type);
        result.extend(&payload);
        result
    }

    /// Decode message from wire format
    pub fn decode(data: &[u8]) -> Result<(Message, usize), ProtocolError> {
        if data.len() < 3 {
            return Err(ProtocolError::InvalidFormat(
                "Message too short".to_string(),
            ));
        }

        let len = u16::from_le_bytes([data[0], data[1]]) as usize;
        if len == 0 {
            return Err(ProtocolError::InvalidFormat(
                "Zero-length message".to_string(),
            ));
        }

        let total_len = 2 + len;
        if data.len() < total_len {
            return Err(ProtocolError::InvalidFormat(format!(
                "Incomplete message: have {} bytes, need {}",
                data.len(),
                total_len
            )));
        }

        let msg_type = data[2];
        let payload = &data[3..total_len];

        let message = match msg_type {
            msg_type::UART_DATA => Message::UartData(payload.to_vec()),
            msg_type::VSYNC => Message::Vsync,
            msg_type::CTS => {
                if payload.is_empty() {
                    return Err(ProtocolError::InvalidFormat(
                        "CTS message missing payload".to_string(),
                    ));
                }
                Message::Cts(payload[0] != 0)
            }
            msg_type::HELLO => {
                if payload.len() < 2 {
                    return Err(ProtocolError::InvalidFormat(
                        "HELLO message too short".to_string(),
                    ));
                }
                Message::Hello {
                    version: payload[0],
                    flags: payload[1],
                }
            }
            msg_type::HELLO_ACK => {
                if payload.is_empty() {
                    return Err(ProtocolError::InvalidFormat(
                        "HELLO_ACK message too short".to_string(),
                    ));
                }
                let version = payload[0];
                let capabilities = String::from_utf8_lossy(&payload[1..]).to_string();
                Message::HelloAck {
                    version,
                    capabilities,
                }
            }
            msg_type::SHUTDOWN => Message::Shutdown,
            _ => return Err(ProtocolError::UnknownMessageType(msg_type)),
        };

        Ok((message, total_len))
    }

    /// Write message to a writer
    pub fn write_to<W: Write>(&self, writer: &mut W) -> Result<(), ProtocolError> {
        let encoded = self.encode();
        writer.write_all(&encoded)?;
        writer.flush()?;
        Ok(())
    }

    /// Read message from a reader
    pub fn read_from<R: Read>(reader: &mut R) -> Result<Message, ProtocolError> {
        // Read length (2 bytes)
        let mut len_buf = [0u8; 2];
        reader.read_exact(&mut len_buf)?;
        let len = u16::from_le_bytes(len_buf) as usize;

        if len == 0 {
            return Err(ProtocolError::InvalidFormat(
                "Zero-length message".to_string(),
            ));
        }

        if len > MAX_UART_DATA_SIZE + 1 {
            return Err(ProtocolError::PayloadTooLarge(len));
        }

        // Read type + payload
        let mut data = vec![0u8; len];
        reader.read_exact(&mut data)?;

        let msg_type = data[0];
        let payload = &data[1..];

        let message = match msg_type {
            msg_type::UART_DATA => Message::UartData(payload.to_vec()),
            msg_type::VSYNC => Message::Vsync,
            msg_type::CTS => {
                if payload.is_empty() {
                    return Err(ProtocolError::InvalidFormat(
                        "CTS message missing payload".to_string(),
                    ));
                }
                Message::Cts(payload[0] != 0)
            }
            msg_type::HELLO => {
                if payload.len() < 2 {
                    return Err(ProtocolError::InvalidFormat(
                        "HELLO message too short".to_string(),
                    ));
                }
                Message::Hello {
                    version: payload[0],
                    flags: payload[1],
                }
            }
            msg_type::HELLO_ACK => {
                if payload.is_empty() {
                    return Err(ProtocolError::InvalidFormat(
                        "HELLO_ACK message too short".to_string(),
                    ));
                }
                let version = payload[0];
                let capabilities = String::from_utf8_lossy(&payload[1..]).to_string();
                Message::HelloAck {
                    version,
                    capabilities,
                }
            }
            msg_type::SHUTDOWN => Message::Shutdown,
            _ => return Err(ProtocolError::UnknownMessageType(msg_type)),
        };

        Ok(message)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_decode_uart_data() {
        let msg = Message::UartData(vec![0x41, 0x42, 0x43]);
        let encoded = msg.encode();
        let (decoded, len) = Message::decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
        assert_eq!(len, encoded.len());
    }

    #[test]
    fn test_encode_decode_vsync() {
        let msg = Message::Vsync;
        let encoded = msg.encode();
        let (decoded, _) = Message::decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_cts() {
        for ready in [true, false] {
            let msg = Message::Cts(ready);
            let encoded = msg.encode();
            let (decoded, _) = Message::decode(&encoded).unwrap();
            assert_eq!(decoded, msg);
        }
    }

    #[test]
    fn test_encode_decode_hello() {
        let msg = Message::Hello {
            version: 1,
            flags: 0,
        };
        let encoded = msg.encode();
        let (decoded, _) = Message::decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_hello_ack() {
        let msg = Message::HelloAck {
            version: 1,
            capabilities: r#"{"type":"cli","cols":80}"#.to_string(),
        };
        let encoded = msg.encode();
        let (decoded, _) = Message::decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_encode_decode_shutdown() {
        let msg = Message::Shutdown;
        let encoded = msg.encode();
        let (decoded, _) = Message::decode(&encoded).unwrap();
        assert_eq!(decoded, msg);
    }

    #[test]
    fn test_wire_format() {
        // Verify exact wire format: [len:u16-LE][type:u8][payload...]
        let msg = Message::UartData(vec![0x41]);
        let encoded = msg.encode();
        // len = 2 (1 byte type + 1 byte payload)
        assert_eq!(encoded, vec![0x02, 0x00, 0x01, 0x41]);
    }
}
