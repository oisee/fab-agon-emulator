# Swift / Go Port Assessment

**Date**: 2026-02-12
**Subject**: Feasibility and complexity of porting fab-agon-emulator from Rust to Swift or Go, including static linking of SDL

## Codebase Size

- **~11,200 lines of Rust** across the workspace (excluding build artifacts and VDP C++ submodules)
- **External `ez80` crate** (CPU instruction decoder/executor) — separate project, pulled from GitHub, would also need porting or wrapping

## Codebase Breakdown by Layer

### 1. eZ80 CPU Emulator (~3,400 lines) — Deepest Part

Pure Rust, zero unsafe code. Implements:
- Memory mapping: 128KB ROM + 512KB external RAM + 8KB on-chip RAM
- I/O port dispatch (~30 unique addresses)
- 6x Programmable Reload Timers
- 2x UARTs with FIFO emulation
- GPIO ports (B, C, D) with interrupt triggering
- SPI SD card interface
- RTC emulation
- "HostFS" — MOS filesystem interception to host OS

This is cycle-accurate and performance-critical. Depends on the external `ez80` crate for actual CPU instruction decode/execute.

### 2. SDL Frontend (~1,500 lines) — Medium Complexity

Main event loop, window/texture rendering, keyboard/mouse/joystick input, audio callback (16384 Hz mono). Straightforward once SDL bindings are available.

### 3. VDP FFI (~200 lines) — Low Complexity

14 `extern "C"` function pointers loaded via `libloading`. Simple to replicate in any language with C FFI.

### 4. Debugger, Protocol, WASM (~2,500 lines) — Optional

Debugger REPL, WebSocket protocol, WASM bindings. Could be deferred or skipped for an initial port.

## Rust-Specific Patterns to Port

- **Traits**: `ez80::Machine` (core abstraction), `SerialLink` (UART), `AudioCallback` — map to protocols (Swift) or interfaces (Go)
- **Concurrency**: `mpsc` channels, `Arc<AtomicBool>` / `Arc<AtomicU8>` for thread-safe shared state, 3 threads (eZ80, VDP, SDL main loop)
- **Pattern matching**: Heavily used in I/O port dispatch — maps to switch/case
- **Enums with data**: `DebugCmd`, `Message` types — maps to enums with associated values (Swift) or tagged unions (Go)
- **No async/await**, no complex generics, only 1 custom macro

## Swift vs Go Comparison

| Aspect | Swift | Go |
|--------|-------|----|
| C/SDL3 interop | Excellent (native bridging) | Good (`cgo`), but per-call overhead |
| Traits → | Protocols (natural fit) | Interfaces (close enough) |
| Threading | GCD / structured concurrency | Goroutines + channels (easy) |
| Performance for CPU emu | Very good (compiled, no GC) | GC pauses could cause timing jitter |
| iOS/tvOS target | **First class** | Not supported |
| SDL3 bindings | Use C API directly | go-sdl2 exists, go-sdl3 less mature |
| Static linking SDL | Xcode handles natively | `cgo` with `-static`, trickier |

## Static Linking SDL

SDL3 supports static linking on all platforms:

- **Swift/Xcode**: Add SDL3 as a build dependency or use prebuilt `.xcframework`. Xcode links statically by default for iOS/tvOS. Path of least resistance.
- **Go**: `CGO_LDFLAGS` with the static `.a` library. Works but `cgo` per-call overhead is a concern for high-frequency FFI (CPU emulator).
- **Rust (current)**: macOS already statically links SDL3 (Makefile `lipo` step).

## Recommendation

**If the goal is iOS/tvOS, Swift is the clear winner:**
- Native Xcode/iOS toolchain, no cross-compilation headaches
- SDL3 C API callable directly, statically linked trivially
- No GC, so CPU emulation performance is solid
- VDP C++ code links naturally as a static `.a`
- ~11K lines Rust → Swift is mechanical (similar semantics)

**Go** is a dead end for iOS/tvOS — Apple doesn't support it on those platforms in any practical way. Only viable for a desktop-only rewrite.

**Alternative: Rust core + Swift shell.** The Rust code cross-compiles to `aarch64-apple-ios`. Keep the Rust core and write a thin Swift wrapper for iOS app lifecycle/UI. The two hard problems (static VDP linking + OpenGL→Metal) exist regardless of language choice.

## Estimated Effort

| Approach | Effort | Notes |
|----------|--------|-------|
| Full Swift rewrite | 6-10 weeks | Including finding/porting an eZ80 CPU library |
| Rust core + Swift iOS shell | 2-4 weeks | Static linking + iOS app wrapper only |
| Full Go rewrite (desktop only) | 6-10 weeks | No iOS/tvOS path |

## Key Risks

1. **The `ez80` CPU emulator crate** — separate project, must be ported or wrapped via FFI
2. **Cycle-accurate timing** — GC languages risk timing jitter in the CPU emulation loop
3. **SDL3 binding maturity** — Rust has mature bindings; Swift/Go bindings are less battle-tested
4. **VDP C++ OpenGL code** — must be solved regardless of language choice (ANGLE/MoltenVK or port to Metal)
