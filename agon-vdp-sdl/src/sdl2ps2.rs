use sdl3;

pub fn is_not_ascii(scancode: sdl3::keyboard::Scancode) -> bool {
    match scancode {
        sdl3::keyboard::Scancode::Backspace |
        sdl3::keyboard::Scancode::Tab |
        sdl3::keyboard::Scancode::CapsLock |
        sdl3::keyboard::Scancode::Return |
        sdl3::keyboard::Scancode::LShift |
        sdl3::keyboard::Scancode::RShift |
        sdl3::keyboard::Scancode::LCtrl |
        sdl3::keyboard::Scancode::LAlt |
        sdl3::keyboard::Scancode::RAlt |
        sdl3::keyboard::Scancode::RCtrl |
        sdl3::keyboard::Scancode::Insert |
        sdl3::keyboard::Scancode::Delete |
        sdl3::keyboard::Scancode::Left |
        sdl3::keyboard::Scancode::Home |
        sdl3::keyboard::Scancode::End |
        sdl3::keyboard::Scancode::Up |
        sdl3::keyboard::Scancode::Down |
        sdl3::keyboard::Scancode::PageUp |
        sdl3::keyboard::Scancode::PageDown |
        sdl3::keyboard::Scancode::Right |
        // numlock
        sdl3::keyboard::Scancode::KpEnter |
        sdl3::keyboard::Scancode::Escape |
        sdl3::keyboard::Scancode::F1 |
        sdl3::keyboard::Scancode::F2 |
        sdl3::keyboard::Scancode::F3 |
        sdl3::keyboard::Scancode::F4 |
        sdl3::keyboard::Scancode::F5 |
        sdl3::keyboard::Scancode::F6 |
        sdl3::keyboard::Scancode::F7 |
        sdl3::keyboard::Scancode::F8 |
        sdl3::keyboard::Scancode::F9 |
        sdl3::keyboard::Scancode::F10 |
        sdl3::keyboard::Scancode::F11 |
        sdl3::keyboard::Scancode::F12 => true,
        _ => false,
    }
}

/**
 * Convert SDL scancodes to PS/2 set 2 scancodes.
 */
pub fn sdl2ps2(scancode: sdl3::keyboard::Scancode, opt_swap_caps_and_ctrl: bool) -> u16 {
    match scancode {
        sdl3::keyboard::Scancode::Grave => 0x0e,
        sdl3::keyboard::Scancode::_1 => 0x16,
        sdl3::keyboard::Scancode::_2 => 0x1e,
        sdl3::keyboard::Scancode::_3 => 0x26,
        sdl3::keyboard::Scancode::_4 => 0x25,
        sdl3::keyboard::Scancode::_5 => 0x2e,
        sdl3::keyboard::Scancode::_6 => 0x36,
        sdl3::keyboard::Scancode::_7 => 0x3d,
        sdl3::keyboard::Scancode::_8 => 0x3e,
        sdl3::keyboard::Scancode::_9 => 0x46,
        sdl3::keyboard::Scancode::_0 => 0x45,
        sdl3::keyboard::Scancode::Minus => 0x4e,
        sdl3::keyboard::Scancode::Equals => 0x55,
        sdl3::keyboard::Scancode::Backspace => 0x66,
        sdl3::keyboard::Scancode::Tab => 0x0d,
        sdl3::keyboard::Scancode::Q => 0x15,
        sdl3::keyboard::Scancode::W => 0x1d,
        sdl3::keyboard::Scancode::E => 0x24,
        sdl3::keyboard::Scancode::R => 0x2d,
        sdl3::keyboard::Scancode::T => 0x2c,
        sdl3::keyboard::Scancode::Y => 0x35,
        sdl3::keyboard::Scancode::U => 0x3C,
        sdl3::keyboard::Scancode::I => 0x43,
        sdl3::keyboard::Scancode::O => 0x44,
        sdl3::keyboard::Scancode::P => 0x4d,
        sdl3::keyboard::Scancode::LeftBracket => 0x54,
        sdl3::keyboard::Scancode::RightBracket => 0x5b,
        sdl3::keyboard::Scancode::CapsLock => {
            if opt_swap_caps_and_ctrl {
                0x14
            } else {
                0x58
            }
        }
        sdl3::keyboard::Scancode::A => 0x1c,
        sdl3::keyboard::Scancode::S => 0x1b,
        sdl3::keyboard::Scancode::D => 0x23,
        sdl3::keyboard::Scancode::F => 0x2b,
        sdl3::keyboard::Scancode::G => 0x34,
        sdl3::keyboard::Scancode::H => 0x33,
        sdl3::keyboard::Scancode::J => 0x3b,
        sdl3::keyboard::Scancode::K => 0x42,
        sdl3::keyboard::Scancode::L => 0x4b,
        sdl3::keyboard::Scancode::Semicolon => 0x4c,
        sdl3::keyboard::Scancode::Apostrophe => 0x52,
        sdl3::keyboard::Scancode::Return => 0x5a,
        sdl3::keyboard::Scancode::LShift => 0x12,
        sdl3::keyboard::Scancode::Z => 0x1a,
        sdl3::keyboard::Scancode::X => 0x22,
        sdl3::keyboard::Scancode::C => 0x21,
        sdl3::keyboard::Scancode::V => 0x2a,
        sdl3::keyboard::Scancode::B => 0x32,
        sdl3::keyboard::Scancode::N => 0x31,
        sdl3::keyboard::Scancode::M => 0x3a,
        sdl3::keyboard::Scancode::Comma => 0x41,
        sdl3::keyboard::Scancode::Period => 0x49,
        sdl3::keyboard::Scancode::Slash => 0x4a,
        sdl3::keyboard::Scancode::RShift => 0x59,
        sdl3::keyboard::Scancode::LCtrl => {
            if opt_swap_caps_and_ctrl {
                0x58
            } else {
                0x14
            }
        }
        sdl3::keyboard::Scancode::LAlt => 0x11,
        sdl3::keyboard::Scancode::Space => 0x29,
        sdl3::keyboard::Scancode::RAlt => 0xe011,
        sdl3::keyboard::Scancode::RCtrl => 0xe014,
        sdl3::keyboard::Scancode::Insert => 0xe070,
        sdl3::keyboard::Scancode::Delete => 0xe071,
        sdl3::keyboard::Scancode::Left => 0xe06b,
        sdl3::keyboard::Scancode::Home => 0xe06c,
        sdl3::keyboard::Scancode::End => 0xe069,
        sdl3::keyboard::Scancode::Up => 0xe075,
        sdl3::keyboard::Scancode::Down => 0xe072,
        sdl3::keyboard::Scancode::PageUp => 0xe07d,
        sdl3::keyboard::Scancode::PageDown => 0xe07a,
        sdl3::keyboard::Scancode::Right => 0xe074,
        sdl3::keyboard::Scancode::NumLockClear => 0x77,
        sdl3::keyboard::Scancode::Kp7 => 0x6c,
        sdl3::keyboard::Scancode::Kp4 => 0x6b,
        sdl3::keyboard::Scancode::Kp1 => 0x69,
        sdl3::keyboard::Scancode::KpDivide => 0xe04a,
        sdl3::keyboard::Scancode::Kp8 => 0x75,
        sdl3::keyboard::Scancode::Kp5 => 0x73,
        sdl3::keyboard::Scancode::Kp2 => 0x72,
        sdl3::keyboard::Scancode::Kp0 => 0x70,
        sdl3::keyboard::Scancode::KpMultiply => 0x7c,
        sdl3::keyboard::Scancode::Kp9 => 0x7d,
        sdl3::keyboard::Scancode::Kp6 => 0x74,
        sdl3::keyboard::Scancode::Kp3 => 0x7a,
        sdl3::keyboard::Scancode::KpPeriod => 0x71,
        sdl3::keyboard::Scancode::KpMinus => 0x7b,
        sdl3::keyboard::Scancode::KpPlus => 0x79,
        sdl3::keyboard::Scancode::KpEnter => 0xe05a,
        sdl3::keyboard::Scancode::Escape => 0x76,
        sdl3::keyboard::Scancode::F1 => 0x05,
        sdl3::keyboard::Scancode::F2 => 0x06,
        sdl3::keyboard::Scancode::F3 => 0x04,
        sdl3::keyboard::Scancode::F4 => 0x0c,
        sdl3::keyboard::Scancode::F5 => 0x03,
        sdl3::keyboard::Scancode::F6 => 0x0b,
        sdl3::keyboard::Scancode::F7 => 0x83,
        sdl3::keyboard::Scancode::F8 => 0x0a,
        sdl3::keyboard::Scancode::F9 => 0x01,
        sdl3::keyboard::Scancode::F10 => 0x09,
        sdl3::keyboard::Scancode::F11 => 0x78,
        sdl3::keyboard::Scancode::F12 => 0x07,
        sdl3::keyboard::Scancode::PrintScreen => 0xe07c, // kinda. good enough for fabgl
        sdl3::keyboard::Scancode::ScrollLock => 0x7e,
        sdl3::keyboard::Scancode::Pause => 0x62,
        // wrong. pause=0x62 is set3, not set2. I use this as pause in set2 is a pain in the arse 8 byte sequence
        sdl3::keyboard::Scancode::Backslash => 0x5d,
        sdl3::keyboard::Scancode::NonUsBackslash => 0x61,
        _ => 0,
    }
}
