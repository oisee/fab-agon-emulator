//! DZRP (DeZog Remote Protocol) message definitions and parsing
//!
//! Protocol format:
//! [4-byte length LE][seq_num (1)][cmd_id (1)][payload...]
//!
//! The length field includes seq_num, cmd_id, and payload (excludes the length field itself)

#![allow(dead_code)]

// DZRP Commands
pub const CMD_INIT: u8 = 1;
pub const CMD_CLOSE: u8 = 2;
pub const CMD_GET_REGISTERS: u8 = 3;
pub const CMD_SET_REGISTER: u8 = 4;
pub const CMD_WRITE_BANK: u8 = 5;
pub const CMD_CONTINUE: u8 = 6;
pub const CMD_PAUSE: u8 = 7;
pub const CMD_READ_MEM: u8 = 8;
pub const CMD_WRITE_MEM: u8 = 9;
pub const CMD_SET_SLOT: u8 = 10;
pub const CMD_GET_TBBLUE_REG: u8 = 11;
pub const CMD_SET_BORDER: u8 = 12;
pub const CMD_SET_BREAKPOINTS: u8 = 13;
pub const CMD_RESTORE_MEM: u8 = 14;
pub const CMD_LOOPBACK: u8 = 15;
pub const CMD_GET_SPRITES_PALETTE: u8 = 16;
pub const CMD_GET_SPRITES_CLIP: u8 = 17;
pub const CMD_GET_SPRITES: u8 = 18;
pub const CMD_GET_SPRITE_PATTERNS: u8 = 19;
pub const CMD_STEP_INTO: u8 = 20;
pub const CMD_READ_STATE: u8 = 21;
pub const CMD_WRITE_STATE: u8 = 22;
pub const CMD_ADD_BREAKPOINT: u8 = 40;
pub const CMD_REMOVE_BREAKPOINT: u8 = 41;
pub const CMD_ADD_WATCHPOINT: u8 = 42;
pub const CMD_REMOVE_WATCHPOINT: u8 = 43;
pub const CMD_STEP_OVER: u8 = 44;
pub const CMD_STEP_OUT: u8 = 45;

// DZRP Notifications (from emulator to DeZog)
pub const NTF_PAUSE: u8 = 1;

// Break reasons for NTF_PAUSE
pub const BREAK_REASON_MANUAL: u8 = 1;
pub const BREAK_REASON_BREAKPOINT: u8 = 2;
pub const BREAK_REASON_WATCHPOINT_READ: u8 = 3;
pub const BREAK_REASON_WATCHPOINT_WRITE: u8 = 4;
pub const BREAK_REASON_OTHER: u8 = 5;

// Breakpoint types
pub const BP_TYPE_PROGRAM: u16 = 0;
pub const BP_TYPE_CONDITION: u16 = 1;
pub const BP_TYPE_LOG: u16 = 2;

/// A DZRP message received from DeZog
#[derive(Debug, Clone)]
pub struct DzrpMessage {
    pub seq_num: u8,
    pub cmd_id: u8,
    pub payload: Vec<u8>,
}

impl DzrpMessage {
    /// Parse a DZRP message from a complete message buffer (excluding length prefix)
    pub fn parse(data: &[u8]) -> Option<Self> {
        if data.len() < 2 {
            return None;
        }
        Some(DzrpMessage {
            seq_num: data[0],
            cmd_id: data[1],
            payload: data[2..].to_vec(),
        })
    }

    /// Create a response message with the same sequence number
    pub fn response(&self, payload: Vec<u8>) -> Vec<u8> {
        let mut response = Vec::with_capacity(6 + payload.len());
        // Length (4 bytes LE) - includes seq_num + response data
        let len = (1 + payload.len()) as u32;
        response.extend_from_slice(&len.to_le_bytes());
        // Sequence number (echoed back)
        response.push(self.seq_num);
        // Payload
        response.extend_from_slice(&payload);
        response
    }
}

/// Create a notification message (no sequence number from client)
pub fn create_notification(ntf_id: u8, payload: &[u8]) -> Vec<u8> {
    let mut msg = Vec::with_capacity(6 + payload.len());
    // Length (4 bytes LE) - includes seq_num (0) + ntf_id + payload
    let len = (2 + payload.len()) as u32;
    msg.extend_from_slice(&len.to_le_bytes());
    // Sequence number 0 for notifications
    msg.push(0);
    // Notification ID
    msg.push(ntf_id);
    // Payload
    msg.extend_from_slice(payload);
    msg
}

/// Read a 16-bit little-endian value from a slice
pub fn read_u16_le(data: &[u8], offset: usize) -> u16 {
    if offset + 2 > data.len() {
        return 0;
    }
    u16::from_le_bytes([data[offset], data[offset + 1]])
}

/// Read a 24-bit little-endian value from a slice (eZ80 addresses)
pub fn read_u24_le(data: &[u8], offset: usize) -> u32 {
    if offset + 3 > data.len() {
        return 0;
    }
    u32::from_le_bytes([data[offset], data[offset + 1], data[offset + 2], 0])
}

/// Read a 32-bit little-endian value from a slice
pub fn read_u32_le(data: &[u8], offset: usize) -> u32 {
    if offset + 4 > data.len() {
        return 0;
    }
    u32::from_le_bytes([
        data[offset],
        data[offset + 1],
        data[offset + 2],
        data[offset + 3],
    ])
}

/// Write a 16-bit little-endian value to a vector
pub fn write_u16_le(vec: &mut Vec<u8>, value: u16) {
    vec.extend_from_slice(&value.to_le_bytes());
}

/// Write a 24-bit little-endian value to a vector (eZ80 addresses)
pub fn write_u24_le(vec: &mut Vec<u8>, value: u32) {
    vec.push((value & 0xFF) as u8);
    vec.push(((value >> 8) & 0xFF) as u8);
    vec.push(((value >> 16) & 0xFF) as u8);
}

/// Write a 32-bit little-endian value to a vector
pub fn write_u32_le(vec: &mut Vec<u8>, value: u32) {
    vec.extend_from_slice(&value.to_le_bytes());
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_message() {
        let data = [0x01, 0x07, 0xAB, 0xCD]; // seq=1, cmd=PAUSE, payload=[0xAB, 0xCD]
        let msg = DzrpMessage::parse(&data).unwrap();
        assert_eq!(msg.seq_num, 1);
        assert_eq!(msg.cmd_id, CMD_PAUSE);
        assert_eq!(msg.payload, vec![0xAB, 0xCD]);
    }

    #[test]
    fn test_response() {
        let msg = DzrpMessage {
            seq_num: 5,
            cmd_id: CMD_INIT,
            payload: vec![],
        };
        let response = msg.response(vec![0x01, 0x02]);
        // Length=3 (seq + 2 bytes payload), seq=5, payload
        assert_eq!(response, vec![0x03, 0x00, 0x00, 0x00, 0x05, 0x01, 0x02]);
    }

    #[test]
    fn test_read_u24_le() {
        let data = [0x12, 0x34, 0x56];
        assert_eq!(read_u24_le(&data, 0), 0x563412);
    }
}
