# Split VDP Architecture: Networked eZ80/VDP Communication

**Date**: 2026-02-03
**Status**: Proposal

## Overview

Proposal to separate the VDP (Video Display Processor) from the eZ80 CPU emulation, allowing them to run as independent processes communicating over IPC or network sockets.

## Motivation

- Run headless eZ80 emulation on servers/CI while displaying on a local machine
- Enable remote "Agon terminal" clients
- Support multiple VDP frontends (native SDL, web-based, etc.)
- Better resource allocation (GPU rendering on capable machine, CPU emulation elsewhere)
- Mirrors real hardware architecture (eZ80 and ESP32 are separate chips with serial link)

## Current Architecture

```
┌─────────────────────────────────────────────────────┐
│                fab-agon-emulator                     │
│  ┌─────────────┐    channels    ┌─────────────────┐ │
│  │ eZ80 Thread │ ◄────────────► │ VDP Thread      │ │
│  │             │   (Sender/     │ (loads .so lib) │ │
│  │ MOS firmware│    Receiver)   │ SDL/OpenGL      │ │
│  └─────────────┘                └─────────────────┘ │
└─────────────────────────────────────────────────────┘
```

## Proposed Architecture

```
┌─────────────────────────┐         ┌─────────────────────────┐
│  agon-cli-emulator      │         │  vdp-server             │
│  (or headless variant)  │         │                         │
│  ┌─────────────┐        │  Unix   │  ┌─────────────────┐    │
│  │ eZ80 CPU    │        │ Socket  │  │ VDP             │    │
│  │             │ ◄──────┼────────►│  │ (loads .so lib) │    │
│  │ MOS firmware│        │  or     │  │ SDL/OpenGL      │    │
│  └─────────────┘        │  TCP    │  └─────────────────┘    │
└─────────────────────────┘         └─────────────────────────┘
      Machine A                           Machine B
      (or same machine)                   (or same machine)
```

## SerialLink Trait

The existing abstraction makes this straightforward:

```rust
// agon-ez80-emulator/src/uart.rs
pub trait SerialLink {
    fn send(&mut self, byte: u8);
    fn recv(&mut self) -> Option<u8>;
    fn read_clear_to_send(&mut self) -> bool;
}
```

## IPC Options (Local Communication)

| Method | Latency | Complexity | Notes |
|--------|---------|------------|-------|
| Shared memory + ring buffer | ~100ns | High | Zero-copy, needs synchronization |
| Unix sockets | ~1-2μs | Low | Best balance for serial data |
| Named pipes (FIFO) | ~1-2μs | Low | Unidirectional only |
| TCP loopback | ~10μs | Low | Cross-platform, network overhead |

**Recommendation**: Unix sockets for local, TCP for remote connections.

## Implementation Plan

### Phase 1: Unix Socket SerialLink

Add to `agon-cli-emulator`:

```rust
use std::os::unix::net::UnixStream;

pub struct UnixSocketSerialLink {
    stream: UnixStream,
    read_buf: VecDeque<u8>,
}

impl SerialLink for UnixSocketSerialLink {
    fn send(&mut self, byte: u8) {
        self.stream.write_all(&[byte]).ok();
    }

    fn recv(&mut self) -> Option<u8> {
        // Non-blocking read from socket
        self.stream.set_nonblocking(true).ok();
        let mut buf = [0u8; 256];
        if let Ok(n) = self.stream.read(&mut buf) {
            self.read_buf.extend(&buf[..n]);
        }
        self.read_buf.pop_front()
    }

    fn read_clear_to_send(&mut self) -> bool {
        true // Flow control could be implemented via socket state
    }
}
```

### Phase 2: VDP Server Binary

New crate: `vdp-server`

```rust
fn main() {
    let socket_path = "/tmp/agon-vdp.sock";
    let listener = UnixListener::bind(socket_path)?;

    // Load VDP .so library (existing code from fab-agon-emulator)
    let vdp = load_vdp_library("firmware/vdp_console8.so")?;

    // Accept connection from eZ80 emulator
    let (stream, _) = listener.accept()?;

    // Bridge socket <-> VDP library
    // - Bytes from socket -> VDP UART receive
    // - VDP UART send -> socket
    // - SDL events -> keyboard packets -> socket
}
```

### Phase 3: CLI Flags

```
agon-cli-emulator [OPTIONS]

OPTIONS:
  --vdp-socket <path>    Connect to VDP via Unix socket
  --vdp-tcp <host:port>  Connect to VDP via TCP
  --vdp-listen <path>    Listen for VDP connection (server mode)
```

```
vdp-server [OPTIONS]

OPTIONS:
  --socket <path>        Listen on Unix socket (default: /tmp/agon-vdp.sock)
  --tcp <port>           Listen on TCP port
  --connect <host:port>  Connect to eZ80 emulator (client mode)
  --firmware <name>      VDP firmware: console8, quark, electron
```

## Use Cases

### Local Split (Same Machine)
```bash
# Terminal 1: VDP server with graphics
vdp-server --socket /tmp/agon.sock

# Terminal 2: Headless eZ80
agon-cli-emulator --vdp-socket /tmp/agon.sock --sdcard ./sdcard
```

### Remote Display
```bash
# Server (headless, runs eZ80):
agon-cli-emulator --vdp-listen 0.0.0.0:5000 --sdcard ./sdcard

# Client (has display):
vdp-server --connect server.local:5000
```

### Web Frontend (Future)
```bash
# eZ80 emulator
agon-cli-emulator --vdp-socket /tmp/agon.sock

# WebSocket bridge + browser-based VDP
vdp-web-server --socket /tmp/agon.sock --http-port 8080
```

## Protocol Considerations

The eZ80-VDP protocol is already defined by the Agon hardware:
- VDP commands: VDU byte sequences
- Keyboard packets: 0x81 + length + keycode + modifiers + vkey + keydown
- System responses: Mode info, RTC, general poll

No new protocol needed - just transport the existing byte stream.

## Future Extensions

1. **Multiplexing**: Multiple VDP clients viewing same eZ80 session
2. **Recording/Replay**: Capture byte stream for debugging
3. **Web VDP**: Browser-based display using WebGL + WebSockets
4. **Hardware bridge**: Connect to real Agon hardware VDP

## Files to Modify/Create

- `agon-cli-emulator/src/main.rs` - Add socket connection options
- `agon-cli-emulator/src/socket_link.rs` - New: Unix/TCP SerialLink implementations
- `vdp-server/` - New crate: Standalone VDP server
- `Cargo.toml` - Add vdp-server to workspace

## References

- Real Agon eZ80-ESP32 communication: Serial UART at 1,152,000 baud
- QEMU `-serial` option: Supports stdio, file, pipe, tcp, unix sockets
- Similar projects: DOSBox serial port networking, VICE remote monitor
