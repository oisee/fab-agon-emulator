# WebSocket VDP Support

**Date**: 2026-02-03
**Status**: Partial Implementation (Work in Progress)

## Overview

Added WebSocket support to the split VDP architecture, enabling browser-based VDP clients to connect to the eZ80 emulator.

## What Was Implemented

### 1. agon-protocol WebSocket Module

Added `websocket.rs` to the agon-protocol crate:

```rust
pub struct WebSocketListener { ... }
pub struct WebSocketConnection { ... }
```

Features:
- `WebSocketListener::bind(port)` - Listen for WebSocket connections
- `WebSocketConnection::send/recv` - Send/receive protocol messages
- `try_recv()` - Non-blocking receive
- Uses `tungstenite` crate for WebSocket handling
- Same message protocol as Unix/TCP sockets

**File**: `agon-protocol/src/websocket.rs`

### 2. agon-ez80 WebSocket Option

Added `--websocket <port>` option to agon-ez80:

```bash
agon-ez80 --websocket 8080 --sdcard ./sdcard
```

- Listens on `ws://0.0.0.0:<port>`
- Alternative to `--socket` (Unix) and `--tcp` options
- Full VDP session handling over WebSocket
- Supports VDP reconnection

**Files**:
- `agon-ez80/src/parse_args.rs` - CLI option
- `agon-ez80/src/main.rs` - WebSocket session handler

### 3. Web VDP Client (Prototype)

Created browser-based VDP client in `web-vdp/`:

```
web-vdp/
├── index.html      # Terminal UI
├── agon-vdp.js     # WebSocket protocol client
└── serve.py        # Simple HTTP server
```

Features:
- xterm.js terminal emulator
- WebSocket connection to agon-ez80
- Protocol handshake (HELLO/HELLO_ACK)
- VSYNC at 60Hz
- Keyboard input queuing with delays
- Basic VDU command parsing

## Current Limitations

### VDP System Commands

The web VDP does not fully implement VDP system command responses. MOS sends queries like:
- `VDU 23, 0, &80, n` - Packet commands
- `VDU 23, 0, &86` - Screen dimensions
- `VDU 23, 0, &94` - Unknown command

The CLI emulator (`agon-vdp-cli`) handles these properly. The web client needs similar implementation.

### Keyboard Input

Keyboard events need to be sent as VDP key packets, not raw ASCII:
```
[packet_length, 0x01, modifiers, ascii, key_down]
```

Currently sending raw bytes which MOS doesn't interpret correctly.

## Usage

```bash
# Terminal 1: Start eZ80 with WebSocket
./target/release/agon-ez80 --websocket 8080 --sdcard ./sdcard

# Terminal 2: Serve web client
cd web-vdp && python3 -m http.server 8000 --bind 0.0.0.0

# Browser: Open http://<host>:8000
# Enter ws://<host>:8080 and click Connect
```

## Architecture

```
┌─────────────────┐      WebSocket       ┌─────────────────┐
│     Browser     │◄────────────────────►│    agon-ez80    │
│                 │    ws://host:8080    │                 │
│  ┌───────────┐  │                      │  ┌───────────┐  │
│  │ xterm.js  │  │   Binary frames:     │  │ eZ80 CPU  │  │
│  │           │  │   [len][type][data]  │  │           │  │
│  └───────────┘  │                      │  │ MOS       │  │
│                 │   HELLO, HELLO_ACK   │  └───────────┘  │
│  ┌───────────┐  │   UART_DATA, VSYNC   │                 │
│  │agon-vdp.js│  │   CTS, SHUTDOWN      │                 │
│  └───────────┘  │                      │                 │
└─────────────────┘                      └─────────────────┘
```

## Dependencies Added

**agon-protocol/Cargo.toml**:
```toml
tungstenite = "0.21"
```

## Files Created/Modified

### New Files
- `agon-protocol/src/websocket.rs` - WebSocket support
- `web-vdp/index.html` - Browser UI
- `web-vdp/agon-vdp.js` - Protocol client
- `web-vdp/serve.py` - HTTP server

### Modified Files
- `agon-protocol/src/lib.rs` - Export WebSocket types
- `agon-protocol/Cargo.toml` - Add tungstenite
- `agon-ez80/src/parse_args.rs` - Add --websocket option
- `agon-ez80/src/main.rs` - WebSocket listener and session handler

## Future Work

1. **Complete VDP command responses** - Implement all VDP system queries
2. **Proper keyboard packets** - Send key events in VDP packet format
3. **Terminal mode** - Handle VDU 23,0,&FF terminal mode switch
4. **Graphics support** - Could use Canvas/WebGL for graphical VDP
5. **Audio** - WebAudio API for sound

## Testing

WebSocket connection works:
- Handshake completes successfully
- VSYNC messages received at 60Hz
- Keyboard input reaches eZ80

MOS boot incomplete:
- MOS sends VDP queries but doesn't receive proper responses
- Need to implement VDP packet responses matching agon-vdp-cli

## References

- agon-vdp-cli/src/text_vdp.rs - Reference VDP implementation
- tungstenite docs: https://docs.rs/tungstenite
- xterm.js: https://xtermjs.org/
