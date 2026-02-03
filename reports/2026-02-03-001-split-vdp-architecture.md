# Split VDP Architecture: Networked eZ80/VDP Communication

**Date**: 2026-02-03
**Status**: Implemented

## Overview

The VDP (Video Display Processor) has been separated from the eZ80 CPU emulation, allowing them to run as independent processes communicating over Unix sockets or TCP.

## Motivation

- Run headless eZ80 emulation on servers/CI while displaying on a local machine
- Enable remote "Agon terminal" clients
- Support multiple VDP frontends (native SDL, web-based, text-only CLI)
- Better resource allocation (GPU rendering on capable machine, CPU emulation elsewhere)
- Mirrors real hardware architecture (eZ80 and ESP32 are separate chips with serial link)
- **VDP reconnection**: VDPs can disconnect and reconnect without restarting eZ80
- **WebSocket support**: Browser-based VDPs can connect (browsers can only connect outward)

## Architecture

The eZ80 acts as the **server** - it listens for VDP connections. VDPs are **clients** that connect to a running eZ80 instance. This allows:
- VDP to reconnect if it crashes or restarts
- Multiple VDP implementations to connect to the same eZ80
- Browser-based WebSocket VDPs (future)

```
┌─────────────────────────┐         ┌─────────────────────────┐
│      agon-ez80          │         │    agon-vdp-cli         │
│      (SERVER)           │  Unix   │      (CLIENT)           │
│  ┌─────────────┐        │ Socket  │  ┌─────────────────┐    │
│  │ eZ80 CPU    │        │   or    │  │ Text VDP        │    │
│  │             │ ◄──────┼────────►│  │ (stdout/stdin)  │    │
│  │ MOS firmware│        │  TCP    │  └─────────────────┘    │
│  └─────────────┘        │         │                         │
│  Listens on socket      │         │  Connects to eZ80       │
└─────────────────────────┘         └─────────────────────────┘
      Machine A                           Machine B
      (or same machine)                   (or same machine)
```

## New Crates

### agon-protocol

Shared protocol library defining message format and socket handling.

```rust
pub enum Message {
    UartData(Vec<u8>),      // 0x01: Bidirectional UART bytes
    Vsync,                   // 0x02: VDP → eZ80 frame sync
    Cts(bool),               // 0x03: VDP → eZ80 flow control
    Hello { version, flags }, // 0x10: Connector → Listener handshake
    HelloAck { version, capabilities }, // 0x11: Listener → Connector response
    Shutdown,                // 0x20: Either direction
}
```

Wire format: `[len:u16-LE][type:u8][payload...]`

### agon-ez80

Standalone eZ80 emulator binary that **listens** for VDP connections.

```bash
agon-ez80 [OPTIONS]
  --socket <path>       Unix socket to listen on (default: /tmp/agon-vdp.sock)
  --tcp <port>          Listen on TCP port instead
  --mos <path>          MOS firmware
  --sdcard <path>       SD card directory
  -u, --unlimited       Unlimited CPU speed
  -d, --debugger        Enable debugger
  -v, -vv, -vvv         Verbosity levels
  --log <file>          Log to file
```

Features:
- CPU starts immediately on launch (no waiting for VDP)
- Accepts VDP connections (one at a time)
- Supports VDP reconnection without restarting CPU

### agon-vdp-cli

Text-only VDP client for terminal/headless operation.

```bash
agon-vdp-cli [OPTIONS]
  --socket <path>       Unix socket to connect to (default: /tmp/agon-vdp.sock)
  --tcp <host:port>     Connect via TCP instead
  -v, -vv, -vvv         Verbosity levels
  --log <file>          Log to file
```

Features:
- Prints VDP text output to stdout
- Reads keyboard input from stdin
- Sends VSYNC at ~60Hz
- Handles VDU commands (color, cursor, system queries)
- Keyboard events sent with 10ms delays (matching real hardware timing)
- Auto-reconnects if connection lost

### agon-vdp-sdl

Graphical VDP client using SDL and the VDP .so library.

```bash
agon-vdp-sdl [OPTIONS]
  --socket <path>       Unix socket to connect to (default: /tmp/agon-vdp.sock)
  --tcp <host:port>     Connect via TCP instead
  -f, --firmware <name> VDP firmware: console8, quark, electron
  --vdp <path>          Explicit path to VDP .so library
  -v, -vv               Verbosity levels
  --fullscreen          Start in fullscreen mode
```

Features:
- Full graphical rendering via SDL3/OpenGL
- Audio via SDL audio subsystem
- Keyboard input via PS/2 scancodes
- Mouse support
- Loads VDP .so firmware (console8, quark, electron)
- Auto-reconnects if connection lost
- Continues rendering during reconnect attempts

## Protocol Details

### Handshake

1. eZ80 listens on socket
2. VDP connects to eZ80
3. VDP sends `HELLO { version: 1, flags: 0 }`
4. eZ80 sends `HELLO_ACK { version: 1, capabilities: "{...}" }`
5. Normal operation begins

The connector (VDP) sends HELLO first, the listener (eZ80) responds with HELLO_ACK.

### Capabilities JSON

VDP capabilities (sent in HELLO):
```json
{"type":"cli","cols":80,"rows":25}
{"type":"sdl","width":640,"height":480,"audio":true}
```

eZ80 capabilities (sent in HELLO_ACK):
```json
{"type":"ez80","version":"1.0"}
```

### Timing

- VSYNC: ~60Hz (16.666ms interval)
- Keyboard events: 10ms delay between each key down/up packet
- UART batching: 100μs intervals

## Usage

### Basic Usage (Two Terminals)

```bash
# Terminal 1: Start eZ80 (server)
./target/debug/agon-ez80 --sdcard ./sdcard

# Terminal 2: Start text VDP (client)
./target/debug/agon-vdp-cli
```

### Graphical VDP

```bash
# Terminal 1: Start eZ80 (server)
./target/debug/agon-ez80 --sdcard ./sdcard

# Terminal 2: Start graphical VDP (client)
./target/debug/agon-vdp-sdl
```

### With Debug Logging

```bash
# Terminal 1: eZ80 with trace logging
./target/debug/agon-ez80 --sdcard ./sdcard -vv --log /tmp/ez80.log

# Terminal 2: VDP with trace logging to file
./target/debug/agon-vdp-cli -vv --log /tmp/vdp.log
```

### TCP Mode (Remote Connection)

```bash
# Machine A: eZ80 listening on TCP
./target/debug/agon-ez80 --tcp 5000 --sdcard ./sdcard

# Machine B: VDP connecting via TCP
./target/debug/agon-vdp-cli --tcp machineA:5000
# or graphical:
./target/debug/agon-vdp-sdl --tcp machineA:5000
```

### VDP Reconnection

If you close and reopen the VDP, it will reconnect to the running eZ80:

```bash
# Start eZ80
./target/debug/agon-ez80 --sdcard ./sdcard

# Start VDP (in another terminal)
./target/debug/agon-vdp-cli
# ... use it, then Ctrl+C

# Reconnect with graphical VDP
./target/debug/agon-vdp-sdl
# Picks up where you left off!
```

## Verified Working

- MOS boot and command prompt
- Directory listing (`DIR`, `ls`)
- Navigation (`cd`)
- CP/M compatibility layer (ZINC)
- Zork I running under ZINC
- VDP reconnection (switch between CLI and SDL VDPs)

Example session:
```
Agon Console8 MOS Version 2.3.3 Rainbow

/ *zinc zork1
ZINC is Not CP/M
(c) 2024 Aleksandr Sharikhin

ZORK I: The Great Underground Empire
Copyright (c) 1981, 1982, 1983 Infocom, Inc.

West of House
You are standing in an open field west of a white house...
```

## Files Created

```
fab-agon-emulator/
├── agon-protocol/           # Protocol library
│   ├── Cargo.toml
│   └── src/
│       ├── lib.rs
│       ├── messages.rs      # Message types & encoding
│       └── socket.rs        # Unix/TCP socket abstraction
│
├── agon-ez80/               # Standalone CPU binary (SERVER)
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── parse_args.rs
│       ├── logger.rs
│       └── socket_link.rs   # SerialLink over socket
│
├── agon-vdp-cli/            # Text VDP client
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── parse_args.rs
│       ├── logger.rs
│       └── text_vdp.rs      # VDU command handling
│
├── agon-vdp-sdl/            # Graphical VDP client
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── parse_args.rs
│       ├── vdp_interface.rs # VDP .so library loading
│       ├── audio.rs         # SDL audio callback
│       └── sdl2ps2.rs       # Keyboard translation
│
├── sdcard_local/            # Local sdcard with ZORK + ZINC
│
└── zinc/                    # ZINC CP/M layer (submodule)
```

## Future Work

- Web-based VDP using WebSockets + Canvas/WebGL
- iOS/Android VDP apps
- Hardware bridge to connect to real Agon VDP
- Session recording/replay for debugging

## References

- ZINC: https://github.com/nihirash/ZINC
- Agon eZ80-ESP32 protocol: Serial UART at 1,152,000 baud
- Similar projects: QEMU serial networking, DOSBox serial port
