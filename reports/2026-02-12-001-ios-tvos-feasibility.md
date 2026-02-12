# iOS / tvOS Build Feasibility Report

**Date**: 2026-02-12
**Subject**: Can fab-agon-emulator be built for iOS (iPad/iPhone) and tvOS?

## Summary

The project already uses SDL3, which has first-class iOS and tvOS backends. The eZ80 CPU emulator (pure Rust) would cross-compile without issues. However, two significant architectural blockers in the VDP layer must be addressed before an iOS/tvOS build is viable.

## Major Blockers

### 1. Dynamic library loading (`libloading` / `dlopen`)

The VDP firmware is loaded at runtime via `libloading` in `src/vdp_interface.rs`. iOS strictly prohibits runtime dynamic library loading -- both at the OS level and as App Store policy.

**Required changes:**
- Compile the VDP C++ code into a static library (`.a`) for `aarch64-apple-ios` / `aarch64-apple-tvos`
- Replace all `libloading::Symbol` lookups with direct `extern "C"` function declarations
- Lose the ability to swap VDP firmware at runtime (or use compile-time feature flags instead)

### 2. OpenGL inside the VDP C++ code

The VDP firmware (FabGL / userspace-vdp-gl) uses OpenGL internally for rendering. SDL3 itself works fine with Metal on iOS, but the VDP C++ code makes direct OpenGL calls. Apple deprecated OpenGL ES on iOS and removed it from newer SDKs.

**Options:**
- Use ANGLE or MoltenVK as a translation layer (OpenGL -> Metal)
- Port the VDP rendering to Metal directly
- Use SDL3's GPU API which abstracts over Metal

## Minor Blockers

### 3. `serialport` dependency

The `serialport` crate will not compile for iOS. Must be feature-gated out for iOS/tvOS targets.

### 4. `raw_tty` dependency

Linux-only already (`cfg(target_os = "linux")`), so not a problem, but similarly any other platform-specific dependencies would need guards.

## What Already Works

| Component | iOS/tvOS Ready? | Notes |
|-----------|-----------------|-------|
| SDL3 (`sdl3 = "0.14.36"`) | Yes | First-class iOS/tvOS backend |
| eZ80 CPU emulator | Yes | Pure Rust, cross-compiles cleanly |
| Rust cross-compilation | Yes | `aarch64-apple-ios`, `aarch64-apple-tvos` targets available |
| Audio | Yes | SDL3 audio works on iOS/tvOS |
| Threading (3-thread arch) | Yes | No issues on iOS |
| Gamepad input | Yes | SDL3 gamepad API, especially relevant for tvOS |

## Implementation Steps

1. **Static-link the VDP** -- Create a build mode that compiles the C++ VDP as a static library and links it directly, replacing the `libloading`-based dynamic loading in `vdp_interface.rs`.

2. **Handle OpenGL -> Metal** -- Integrate ANGLE or MoltenVK to translate the VDP's OpenGL calls to Metal, or port to SDL3's GPU API.

3. **Feature-gate platform-specific deps** -- Guard `serialport`, `raw_tty`, and any other desktop-only crates behind `cfg` attributes.

4. **Xcode project wrapper** -- SDL3 on iOS requires an Xcode project to produce the `.app` bundle, handle code signing, provisioning, etc.

5. **Input adaptation**:
   - On-screen keyboard or Bluetooth keyboard support for the console
   - Touch-to-mouse mapping
   - Game controller support (especially important for tvOS) via SDL3's gamepad API

6. **SD card -> app sandbox** -- Map the SD card directory to the app's Documents folder, with possible Files app integration for loading programs.

## Conclusion

The project is well-positioned for an iOS/tvOS port thanks to SDL3 and the pure-Rust CPU emulator. The work is concentrated in the VDP layer: replacing dynamic loading with static linking, and solving the OpenGL-on-Metal problem. The static-linking change would also benefit a potential WebAssembly target.
