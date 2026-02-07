# Frame Dump Feature for agon-vdp-sdl

**Date**: 2026-02-07
**Status**: Complete

## Overview

Added the ability to capture VDP framebuffer contents as PNG files for debugging rendering issues. This is particularly useful for investigating timing-sensitive bugs such as the yellow screen glitch, where the VDP cannot process large UART payloads within one or two vsync cycles.

## Motivation

When debugging VDP rendering, it is often necessary to see exactly what the framebuffer contains at each vsync boundary. Two common scenarios:

1. **Full capture** — record every single frame to create a complete timeline of rendering activity
2. **Keyframe capture** — record only frames where the eZ80 sent drawing commands via UART, filtering out idle frames to reduce output volume

## CLI Flags

| Flag | Behavior |
|------|----------|
| `--dump-frames <dir>` | Save **every** frame as PNG on each vsync |
| `--dump-keyframes <dir>` | Save frame **only** when UART data arrived since last vsync |

Both produce sequentially numbered PNGs: `frame_000001.png`, `frame_000002.png`, etc. The output directory is created automatically if it does not exist.

## Implementation

### Files Modified

#### `agon-vdp-sdl/Cargo.toml`
- Added `png = "0.17"` dependency for PNG encoding

#### `agon-vdp-sdl/src/parse_args.rs`
- Added `dump_frames: Option<String>` and `dump_keyframes: Option<String>` fields to `AppArgs`
- Added `--dump-frames <dir>` and `--dump-keyframes <dir>` argument parsing
- Updated help text

#### `agon-vdp-sdl/src/main.rs`
- Added `save_frame_png()` helper function that encodes RGB24 framebuffer data to PNG using the `png` crate with `BufWriter` for efficient I/O
- Added `uart_had_activity` flag, set to `true` on each `Message::UartData` receipt
- Added `dump_frame_num` counter for sequential filenames
- After `copyVgaFramebuffer()`, conditionally dumps the frame:
  - `--dump-frames`: dumps every frame unconditionally
  - `--dump-keyframes`: dumps only when `uart_had_activity` is true
- Resets `uart_had_activity` after each vsync dump check

### Design Decisions

- **RGB24 format**: The framebuffer is already in RGB24 layout from `copyVgaFramebuffer()`, so no pixel format conversion is needed
- **Sequential numbering**: 6-digit zero-padded filenames (`frame_000001.png`) sort naturally and support up to 999,999 frames (over 4.5 hours at 60fps)
- **Lazy directory creation**: The output directory is created on first write via `create_dir_all`, so the user doesn't need to pre-create it
- **Error resilience**: PNG write failures are logged to stderr but do not terminate the emulator

## Usage

```bash
# Dump all frames
./target/release/agon-vdp-sdl --dump-frames /tmp/frames

# Dump only frames where eZ80 sent UART data
./target/release/agon-vdp-sdl --dump-keyframes /tmp/keyframes

# Inspect output
ls /tmp/frames/       # frame_000001.png, frame_000002.png, ...
ls /tmp/keyframes/    # fewer files — only when drawing commands were sent
```

## Debugging Workflow Example

To debug the yellow screen glitch:

1. Start `agon-ez80` with a program that triggers the issue
2. Start `agon-vdp-sdl --dump-keyframes /tmp/debug`
3. Reproduce the issue
4. Examine the PNGs in `/tmp/debug/` to see exactly what the VDP rendered on each frame where UART data was received
5. Correlate frame numbers with UART trace output (`-vv`) to identify timing issues
