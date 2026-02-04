# WASM Emulator Implementation

**Date**: 2026-02-03
**Status**: Functional Prototype

## Overview

Compiled the Agon eZ80 emulator to WebAssembly, enabling it to run directly in web browsers without a server-side component.

## What Was Implemented

### 1. agon-wasm Crate

New workspace member `agon-wasm/` containing a minimal eZ80 emulator suitable for WASM compilation.

**Key features:**
- Uses the `ez80` crate for CPU emulation
- Implements the `Machine` trait for memory and I/O
- Exposes WASM-friendly API via `wasm-bindgen`
- No file I/O or system dependencies

**Files:**
- `agon-wasm/Cargo.toml` - Dependencies: wasm-bindgen, js-sys, web-sys, ez80
- `agon-wasm/src/lib.rs` - Main emulator implementation

### 2. WASM API

```rust
#[wasm_bindgen]
pub struct AgonEmulator { ... }

impl AgonEmulator {
    pub fn new() -> AgonEmulator;
    pub fn load_mos(&mut self, data: &[u8]);
    pub fn run_cycles(&mut self, max_cycles: u32) -> u32;
    pub fn send_byte(&mut self, byte: u8);
    pub fn send_key(&mut self, ascii: u8, down: bool);
    pub fn get_output(&mut self) -> Vec<u8>;
    pub fn has_output(&self) -> bool;
    pub fn get_cycles(&self) -> u64;
    pub fn reset(&mut self);
}
```

### 3. Web Frontend

Created `web-emu/` directory with browser-based emulator interface:

- **index.html** - Main UI with xterm.js terminal
- **pkg/** - Generated WASM package (wasm-pack output)
- **MOS.bin** - Optional MOS firmware for auto-loading

**Features:**
- xterm.js terminal for text output
- VDU command parser for display
- VDP system command emulation
- Keyboard input as VDP key packets
- File picker for loading MOS firmware
- Auto-loads MOS.bin if present

## Memory Map

The WASM emulator implements the Agon memory map:

| Address Range | Size | Description |
|--------------|------|-------------|
| 0x000000 - 0x01FFFF | 128 KB | ROM (MOS firmware) |
| 0x040000 - 0x0BFFFF | 512 KB | External RAM |
| 0x0BC000 - 0x0BDFFF | 8 KB | Internal RAM |

## I/O Ports

Emulates UART0 for eZ80/VDP communication:

| Port | Function |
|------|----------|
| 0xC0 | UART0 RBR/THR (receive/transmit buffer) |
| 0xC1 | UART0 IER (interrupt enable) |
| 0xC2 | UART0 IIR/FCR |
| 0xC3 | UART0 LCR (line control) |
| 0xC5 | UART0 LSR (line status) |
| 0x9A | GPIO Port B |

## VDP Emulation

The JavaScript frontend implements basic VDP responses:

- Screen dimensions (80x25)
- Cursor position tracking
- RTC (current time)
- Mode information
- Keyboard layout

**VDU Commands Handled:**
- VDU 8 (backspace)
- VDU 10 (line feed)
- VDU 12 (clear screen)
- VDU 13 (carriage return)
- VDU 23,0,n (system commands)
- VDU 31,x,y (TAB)
- Printable ASCII (0x20-0x7E)

## Building

```bash
# Build WASM package
cd agon-wasm
wasm-pack build --target web

# Copy to web directory
cp -r pkg ../web-emu/

# Serve
cd ../web-emu
python3 -m http.server 8001
```

## Limitations

1. **No Graphics Mode**: Only text terminal output
2. **No Audio**: Sound not implemented
3. **No SD Card**: File system not available
4. **Limited VDP**: Only basic system queries implemented
5. **Keyboard**: Simple ASCII only, no special keys

## Architecture

```
┌─────────────────────────────────────────────────────────┐
│                       Browser                           │
│  ┌─────────────┐    ┌──────────────┐    ┌───────────┐  │
│  │  index.html │◄──►│ agon_wasm.js │◄──►│  WASM     │  │
│  │             │    │              │    │ (eZ80 CPU)│  │
│  │  ┌───────┐  │    │ VDU Parser   │    │           │  │
│  │  │xterm.js│ │    │ VDP Emulator │    │ Memory    │  │
│  │  └───────┘  │    │ Key Handler  │    │ UART I/O  │  │
│  └─────────────┘    └──────────────┘    └───────────┘  │
└─────────────────────────────────────────────────────────┘
```

## Dependencies

**Rust crates:**
- `wasm-bindgen` 0.2 - JavaScript bindings
- `js-sys` 0.3 - JavaScript types
- `web-sys` 0.3 - Web APIs
- `ez80` - CPU emulation
- `console_error_panic_hook` 0.1 - Better panic messages

**JavaScript:**
- xterm.js - Terminal emulator

## Testing

Open http://localhost:8001/ in a browser:

1. WASM module loads automatically
2. MOS.bin loads if present, or use file picker
3. Click "Start" to begin emulation
4. Type to send keyboard input
5. MOS boot messages appear in terminal

## Future Work

1. **Graphics Canvas**: Use Canvas/WebGL for graphical modes
2. **WebAudio**: Add audio output
3. **IndexedDB**: Implement virtual file system
4. **Full Keyboard**: Add modifier keys, function keys
5. **Debug Interface**: Add breakpoints, memory viewer
6. **Mobile Support**: Touch keyboard

## Files Created

- `agon-wasm/Cargo.toml`
- `agon-wasm/src/lib.rs`
- `web-emu/index.html`
- `web-emu/pkg/*` (generated)
- `web-emu/MOS.bin` (copied from sdcard)

## References

- wasm-bindgen: https://rustwasm.github.io/wasm-bindgen/
- xterm.js: https://xtermjs.org/
- ez80 datasheet: https://www.zilog.com/docs/ez80/UM0077.pdf
