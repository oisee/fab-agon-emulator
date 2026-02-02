# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

Fab Agon Emulator is an emulator for the Agon Light, Agon Light 2, and Agon Console8 8-bit computers. It emulates both the eZ80 CPU (running MOS firmware) and the ESP32-based VDP (Video Display Processor).

**Languages**: Rust (main emulator) + C++ (VDP firmware)

## Build Commands

```bash
# Initialize submodules (required before first build)
git submodule update --init

# Build everything (VDP shared libraries + Rust binary)
make

# Clean all build artifacts
make clean

# Install to system prefix
PREFIX=/usr/local make && sudo make install

# Run the emulator
./fab-agon-emulator
```

**Important**: Do not run `cargo build` directly. The Makefile sets `FORCE=1` to bypass a check that prevents direct cargo builds (the VDP libraries must be built first).

## Architecture

The emulator uses a three-thread architecture:

1. **EZ80 CPU Thread** - Emulates the Zilog eZ80 processor at 18.432 MHz, runs MOS firmware, handles GPIO for joystick input
2. **VDP Thread** - Runs dynamically loaded C++ library (.so), handles graphics via OpenGL and audio
3. **SDL Event Loop** (main thread) - Manages window rendering, keyboard/mouse/joystick input, copies VGA framebuffer from VDP

### Inter-thread Communication
- **Z80 ↔ VDP**: UART0 serial link via FFI symbol functions
- **VDP → SDL**: Shared memory framebuffer (copyVgaFramebuffer), audio samples
- **SDL → VDP**: Keyboard/mouse events via FFI

### Key Source Files

| File | Purpose |
|------|---------|
| `src/main.rs` | Entry point, thread orchestration, SDL event loop |
| `src/vdp_interface.rs` | Dynamic loading of VDP .so libraries, FFI definitions |
| `src/parse_args.rs` | Command-line argument parsing |
| `src/audio.rs` | SDL2 audio callback (16384 Hz mono) |
| `src/joypad.rs` | Joystick/gamepad to GPIO mapping |
| `src/ez80_serial_links.rs` | UART communication between Z80 and VDP/host |
| `src/sdl2ps2.rs` | SDL2 scancode to PS/2 conversion |

### VDP Firmware (src/vdp/)

The VDP is compiled as C++ shared libraries from submodules:
- `userspace-vdp-gl/` - FabGL 1.0.8 fork (graphics engine)
- `vdp-console8/` - Console8 VDP firmware (default)
- `vdp-quark/` - Quark 1.04 firmware
- `AgonElectronHAL/` - ElectronOS HAL

Build produces: `firmware/vdp_*.so`

### CPU Emulator (agon-ez80-emulator/)

The eZ80 emulation is a local workspace crate. Key files:
- `agon_machine.rs` (1600+ lines) - main emulation logic, I/O handling
- `gpio.rs` - GPIO ports emulation
- `prt_timer.rs` - programmable reload timers
- `debugger.rs` - debug interface

### Debugger

The Z80 debugger is a workspace member at `agon-light-emulator-debugger/`. Enable with `-d` or `--debugger` flag. Supports breakpoints (`-b 0x1234`), CPU state inspection, and UART1 serial testing.

### CLI Emulator (agon-cli-emulator/)

Headless emulator for automated testing and regression tests. Run with:
```bash
cargo build -r --manifest-path=./agon-cli-emulator/Cargo.toml
./target/release/agon-cli-emulator
```

## Platform-Specific Notes

- **Linux**: Uses system SDL2 via pkg-config
- **macOS**: Builds arm64 binary, SDL2 bundled and statically linked
- **Windows**: Must use MSYS2 UCRT64 environment, run `msys-init.sh` first

## Large Files

Avoid reading these files in full - they are large and contain repetitive patterns:
- `agon-ez80-emulator/src/agon_machine.rs` - 1600+ lines, main CPU loop
- `src/vdp/userspace-vdp-gl/` - FabGL submodule, large C++ codebase
- `src/ascii2vk.rs` - keyboard mapping tables

## Running the Emulator

```bash
# Default (Console8 firmware)
./fab-agon-emulator

# With different firmware
./fab-agon-emulator --firmware quark
./fab-agon-emulator --firmware electron

# With debugger
./fab-agon-emulator -d

# Custom SD card directory
./fab-agon-emulator --sdcard <dir>

# Unlimited CPU speed
./fab-agon-emulator -u

# Custom MOS/VDP firmware
./fab-agon-emulator --mos path/to/mos.bin --vdp path/to/vdp.so
```

## Keyboard Shortcuts (Right Ctrl + key)

- `C` - Toggle caps-lock
- `F` - Toggle fullscreen
- `M` - Print ESP32 memory stats
- `R` - Soft reset
- `S` - Cycle screen scaling
- `Q` - Quit
