//! Translates between DZRP protocol and internal DebugCmd/DebugResp types

#![allow(dead_code)]

use crate::protocol::*;
use agon_ez80_emulator::debugger::{DebugCmd, DebugResp, PauseReason, Reg8, Reg16, Registers, Trigger};

/// eZ80 register indices as used in DZRP
/// The register format for eZ80 is 38 bytes:
/// PC(3), SP(3), AF(2), BC(3), DE(3), HL(3), IX(3), IY(3),
/// AF'(2), BC'(3), DE'(3), HL'(3), I(1), R(1), IM(1), ADL(1)
pub const REG_SIZE: usize = 38;

/// Register indices for SET_REGISTER command (extended for eZ80)
pub const REG_PC: u8 = 0;
pub const REG_SP: u8 = 1;
pub const REG_AF: u8 = 2;
pub const REG_BC: u8 = 3;
pub const REG_DE: u8 = 4;
pub const REG_HL: u8 = 5;
pub const REG_IX: u8 = 6;
pub const REG_IY: u8 = 7;
pub const REG_AF2: u8 = 8;  // AF'
pub const REG_BC2: u8 = 9;  // BC'
pub const REG_DE2: u8 = 10; // DE'
pub const REG_HL2: u8 = 11; // HL'
pub const REG_I: u8 = 12;
pub const REG_R: u8 = 13;
pub const REG_IM: u8 = 14;

/// Convert a DZRP command to internal DebugCmd(s)
/// Returns None if the command is not supported or invalid
pub fn dzrp_to_debug_cmd(msg: &DzrpMessage) -> Option<Vec<DebugCmd>> {
    match msg.cmd_id {
        CMD_INIT => {
            // INIT doesn't require a debug command, handled directly
            None
        }
        CMD_CLOSE => {
            // Close is handled at server level
            None
        }
        CMD_GET_REGISTERS => {
            Some(vec![DebugCmd::GetRegisters])
        }
        CMD_SET_REGISTER => {
            // Payload: [reg_index, value...]
            // Value size depends on register (2 or 3 bytes for eZ80)
            if msg.payload.is_empty() {
                return None;
            }
            let reg_index = msg.payload[0];
            let value = if msg.payload.len() >= 4 {
                read_u24_le(&msg.payload, 1)
            } else if msg.payload.len() >= 3 {
                read_u16_le(&msg.payload, 1) as u32
            } else {
                return None;
            };
            Some(vec![DebugCmd::SetRegister { reg_index, value }])
        }
        CMD_CONTINUE => {
            Some(vec![DebugCmd::Continue])
        }
        CMD_PAUSE => {
            Some(vec![DebugCmd::Pause(PauseReason::DebuggerRequested)])
        }
        CMD_READ_MEM => {
            // Payload: [start (3 bytes), len (2 bytes)]
            if msg.payload.len() < 5 {
                return None;
            }
            let start = read_u24_le(&msg.payload, 0);
            let len = read_u16_le(&msg.payload, 3) as u32;
            Some(vec![DebugCmd::GetMemory { start, len }])
        }
        CMD_WRITE_MEM => {
            // Payload: [start (3 bytes), data...]
            if msg.payload.len() < 3 {
                return None;
            }
            let start = read_u24_le(&msg.payload, 0);
            let data = msg.payload[3..].to_vec();
            Some(vec![DebugCmd::WriteMemory { start, data }])
        }
        CMD_STEP_INTO => {
            Some(vec![DebugCmd::Step])
        }
        CMD_STEP_OVER => {
            Some(vec![DebugCmd::StepOver])
        }
        CMD_ADD_BREAKPOINT => {
            // Payload: [bp_id (2 bytes), bp_type (2 bytes), address (3 bytes), ...]
            if msg.payload.len() < 7 {
                return None;
            }
            let address = read_u24_le(&msg.payload, 4);
            let trigger = Trigger {
                address,
                once: false,
                actions: vec![
                    DebugCmd::Pause(PauseReason::DebuggerBreakpoint),
                    DebugCmd::GetState,
                ],
            };
            Some(vec![DebugCmd::AddTrigger(trigger)])
        }
        CMD_REMOVE_BREAKPOINT => {
            // Payload: [address (3 bytes)]
            if msg.payload.len() < 3 {
                return None;
            }
            let address = read_u24_le(&msg.payload, 0);
            Some(vec![DebugCmd::DeleteTrigger(address)])
        }
        CMD_LOOPBACK => {
            // Loopback - just echo back, no debug command needed
            None
        }
        _ => {
            // Unsupported command
            None
        }
    }
}

/// Convert internal registers to DZRP register format (38 bytes for eZ80)
pub fn registers_to_dzrp(reg: &Registers) -> Vec<u8> {
    let mut data = Vec::with_capacity(REG_SIZE);

    // PC (3 bytes)
    write_u24_le(&mut data, reg.pc);

    // SP (3 bytes) - use 24-bit if in ADL mode, else 16-bit with MBASE
    let sp = if reg.adl {
        reg.get24(Reg16::SP)
    } else {
        reg.get16_mbase(Reg16::SP)
    };
    write_u24_le(&mut data, sp);

    // AF (2 bytes - always 16-bit)
    write_u16_le(&mut data, reg.get16(Reg16::AF));

    // BC (3 bytes)
    write_u24_le(&mut data, reg.get24(Reg16::BC));

    // DE (3 bytes)
    write_u24_le(&mut data, reg.get24(Reg16::DE));

    // HL (3 bytes)
    write_u24_le(&mut data, reg.get24(Reg16::HL));

    // IX (3 bytes)
    write_u24_le(&mut data, reg.get24(Reg16::IX));

    // IY (3 bytes)
    write_u24_le(&mut data, reg.get24(Reg16::IY));

    // AF' (2 bytes) - alternate registers not accessible via ez80 public API, return 0
    write_u16_le(&mut data, 0);

    // BC' (3 bytes)
    write_u24_le(&mut data, 0);

    // DE' (3 bytes)
    write_u24_le(&mut data, 0);

    // HL' (3 bytes)
    write_u24_le(&mut data, 0);

    // I (1 byte)
    data.push(reg.get8(Reg8::I));

    // R (1 byte)
    data.push(reg.get8(Reg8::R));

    // IM (1 byte) - interrupt mode (not accessible via ez80 public API)
    data.push(0);

    // ADL (1 byte) - ADL mode flag
    data.push(if reg.adl { 1 } else { 0 });

    data
}

/// Convert DebugResp to DZRP response payload
pub fn debug_resp_to_dzrp(resp: &DebugResp) -> Option<Vec<u8>> {
    match resp {
        DebugResp::Pong => {
            Some(vec![])
        }
        DebugResp::Resumed => {
            Some(vec![])
        }
        DebugResp::Registers(reg) => {
            Some(registers_to_dzrp(reg))
        }
        DebugResp::Memory { data, .. } => {
            Some(data.clone())
        }
        DebugResp::State { registers, .. } => {
            // For GET_REGISTERS, just return register data
            Some(registers_to_dzrp(registers))
        }
        DebugResp::Paused(reason) => {
            // Paused responses are handled as notifications
            Some(pause_to_notification_payload(reason, 0))
        }
        _ => None,
    }
}

/// Convert a PauseReason to NTF_PAUSE notification payload
pub fn pause_to_notification_payload(reason: &PauseReason, pc: u32) -> Vec<u8> {
    let mut payload = Vec::with_capacity(4);

    // Break reason
    let break_reason = match reason {
        PauseReason::DebuggerRequested => BREAK_REASON_MANUAL,
        PauseReason::DebuggerBreakpoint => BREAK_REASON_BREAKPOINT,
        PauseReason::IOBreakpoint(_) => BREAK_REASON_OTHER,
        PauseReason::OutOfBoundsMemAccess(_) => BREAK_REASON_OTHER,
    };
    payload.push(break_reason);

    // PC (3 bytes LE)
    write_u24_le(&mut payload, pc);

    payload
}

/// Create the INIT response payload
/// Returns machine type info for eZ80
pub fn create_init_response() -> Vec<u8> {
    let mut payload = Vec::new();

    // Error code (1 byte) - 0 = success
    payload.push(0);

    // DZRP version (1 byte)
    payload.push(2);

    // Machine type name (string with length prefix)
    let machine_name = b"eZ80";
    payload.push(machine_name.len() as u8);
    payload.extend_from_slice(machine_name);

    // Extended capabilities (optional, but good to have)
    // Number of breakpoints available
    write_u16_le(&mut payload, 255);

    payload
}
