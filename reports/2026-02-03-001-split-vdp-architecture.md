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

## Architecture

```
┌─────────────────────────┐         ┌─────────────────────────┐
│      agon-ez80          │         │    agon-vdp-cli         │
│                         │  Unix   │                         │
│  ┌─────────────┐        │ Socket  │  ┌─────────────────┐    │
│  │ eZ80 CPU    │        │   or    │  │ Text VDP        │    │
│  │             │ ◄──────┼────────►│  │ (stdout/stdin)  │    │
│  │ MOS firmware│        │  TCP    │  └─────────────────┘    │
│  └─────────────┘        │         │                         │
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
    Hello { version, flags }, // 0x10: eZ80 → VDP handshake
    HelloAck { version, capabilities }, // 0x11: VDP → eZ80 handshake
    Shutdown,                // 0x20: Either direction
}
```

Wire format: `[len:u16-LE][type:u8][payload...]`

### agon-ez80

Standalone eZ80 emulator binary that connects to an external VDP.

```bash
agon-ez80 [OPTIONS]
  --socket <path>       Unix socket (default: /tmp/agon-vdp.sock)
  --tcp <host:port>     Use TCP instead
  --mos <path>          MOS firmware
  --sdcard <path>       SD card directory
  -u, --unlimited       Unlimited CPU speed
  -d, --debugger        Enable debugger
  -v, -vv, -vvv         Verbosity levels
  --log <file>          Log to file
```

### agon-vdp-cli

Text-only VDP server for terminal/headless operation.

```bash
agon-vdp-cli [OPTIONS]
  --socket <path>       Unix socket (default: /tmp/agon-vdp.sock)
  --tcp <port>          Listen on TCP
  -v, -vv, -vvv         Verbosity levels
  --log <file>          Log to file
```

Features:
- Prints VDP text output to stdout
- Reads keyboard input from stdin
- Sends VSYNC at ~60Hz
- Handles VDU commands (color, cursor, system queries)
- Keyboard events sent with 10ms delays (matching real hardware timing)

## Protocol Details

### Handshake

1. VDP listens on socket
2. eZ80 connects
3. eZ80 sends `HELLO { version: 1, flags: 0 }`
4. VDP sends `HELLO_ACK { version: 1, capabilities: "{...}" }`
5. Normal operation begins

### Capabilities JSON

```json
{"type":"cli","cols":80,"rows":25}
```

Future VDPs might report:
```json
{"type":"gfx","width":640,"height":480,"audio":true}
```

### Timing

- VSYNC: ~60Hz (16.666ms interval)
- Keyboard events: 10ms delay between each key down/up packet
- UART batching: 100μs intervals

## Usage

### Basic Usage (Two Terminals)

```bash
# Terminal 1: Start text VDP
./target/debug/agon-vdp-cli

# Terminal 2: Start eZ80
./target/debug/agon-ez80 --sdcard ./sdcard
```

### With Debug Logging

```bash
# Terminal 1: VDP with trace logging to file
./target/debug/agon-vdp-cli -vv --log /tmp/vdp.log

# Terminal 2: eZ80 with trace logging
./target/debug/agon-ez80 --sdcard ./sdcard -vv --log /tmp/ez80.log
```

### TCP Mode (Remote Connection)

```bash
# Machine A: VDP listening on TCP
./target/debug/agon-vdp-cli --tcp 5000

# Machine B: eZ80 connecting via TCP
./target/debug/agon-ez80 --tcp machineA:5000 --sdcard ./sdcard
```

## Verified Working

- MOS boot and command prompt
- Directory listing (`DIR`, `ls`)
- Navigation (`cd`)
- CP/M compatibility layer (ZINC)
- Zork I running under ZINC

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
├── agon-ez80/               # Standalone CPU binary
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── parse_args.rs
│       ├── logger.rs
│       └── socket_link.rs   # SerialLink over socket
│
├── agon-vdp-cli/            # Text VDP server
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs
│       ├── parse_args.rs
│       ├── logger.rs
│       └── text_vdp.rs      # VDU command handling
│
└── zinc/                    # ZINC CP/M layer (submodule)
```

## Future Work

- `agon-vdp-bridge`: Graphical VDP wrapping the C++ .so library
- Web-based VDP using WebSockets + Canvas/WebGL
- iOS/Android VDP apps
- Hardware bridge to connect to real Agon VDP

## References

- ZINC: https://github.com/nihirash/ZINC
- Agon eZ80-ESP32 protocol: Serial UART at 1,152,000 baud
- Similar projects: QEMU serial networking, DOSBox serial port
