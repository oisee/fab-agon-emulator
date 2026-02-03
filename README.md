# Fab Agon Emulator

An emulator of the Agon Light, Agon Light 2, and Agon Console8 8-bit computers.

## Binaries

This project provides multiple emulator binaries:

| Binary | Description |
|--------|-------------|
| `fab-agon-emulator` | Full graphical emulator with SDL2/OpenGL |
| `agon-cli-emulator` | Headless CLI emulator (combined eZ80+VDP) |
| `agon-ez80` | Standalone eZ80 CPU (connects to external VDP) |
| `agon-vdp-cli` | Text-only VDP server (for terminal use) |

The split architecture (`agon-ez80` + `agon-vdp-cli`) allows running the CPU and display on different machines or in different processes. See [Split VDP Architecture](./reports/2026-02-03-001-split-vdp-architecture.md) for details.

## How to compile

You may not need to compile, as there are regular pre-compiled
[releases](https://github.com/tomm/fab-agon-emulator/releases)
for Linux (amd64), Windows (x64) and Mac (Intel & ARM).

Otherwise, read the [guide to compiling Fab Agon Emulator](./docs/compiling.md)

## Quick Start: Split Architecture

Run the eZ80 and VDP as separate processes:

```bash
# Build the split binaries
cargo build -p agon-ez80 -p agon-vdp-cli

# Terminal 1: Start the text VDP
./target/debug/agon-vdp-cli

# Terminal 2: Start the eZ80 CPU
./target/debug/agon-ez80 --sdcard ./sdcard
```

This gives you a text-only Agon terminal. You can run MOS commands, BASIC programs, and even CP/M software via ZINC:

```
/ *zinc zork1
ZORK I: The Great Underground Empire
West of House
You are standing in an open field west of a white house...
```

## Keyboard Shortcuts

Emulator shortcuts are accessed with the *right ctrl*.

 * RightCtrl-C - Toggle caps-lock
 * RightCtrl-F - Toggle fullscreen mode
 * RightCtrl-M - Print ESP32 memory stats to the console
 * RightCtrl-R - Soft-reset
 * RightCtrl-S - Cycle screen scaling methods (see --scale command line option)
 * RightCtrl-Q - Quit
 * RightCtrl-1 - Show VDP video output
 * RightCtrl-2 - Show GPIO video output if available, or VDP output otherwise

## Emulated SDCard

If a directory is specified with `fab-agon-emulator --sdcard <dir>` then that will
be used as the emulated SDCard. Otherwise, the `.agon-sdcard/` directory in your
home directory will be used if present, and if not then `sdcard/` in the current
directory is used.

Alternatively you can use SDCard images (full MBR partitioned images, or raw
FAT32 images), with the --sdcard-img option.

## Changing VDP version

By default, Fab Agon Emulator boots with Console8 firmware. To start up
with quark firmware, run:

```
fab-agon-emulator --firmware quark
```

Legacy 1.03 firmware is also available:

```
fab-agon-emulator --firmware 1.03
```

And Electron firmware:

```
fab-agon-emulator --firmware electron
```

## The Z80 debugger

Start the emulator with the `-d` or `--debugger` option to enable the Z80
debugger:

```
fab-agon-emulator -d
```

At the debugger prompt (which will be in the terminal window you invoked the
emulator from), type `help` for instructions on the use of the debugger.

## DeZog Integration (DZRP)

The emulator supports the DeZog Remote Protocol (DZRP) for VS Code debugging.
Start with:

```
fab-agon-emulator --dzrp
```

This opens a TCP server on port 11000 (configurable with `--dzrp-port`).

To use with VS Code:
1. Install the [DeZog extension](https://marketplace.visualstudio.com/items?itemName=maziac.dezog)
2. Create `.vscode/launch.json`:
```json
{
  "version": "0.2.0",
  "configurations": [{
    "type": "dezog",
    "request": "launch",
    "name": "Agon DZRP",
    "remoteType": "dzrp",
    "dzrp": { "port": 11000 }
  }]
}
```
3. Start the emulator with `--dzrp`, then launch the DeZog debugger

Note: `--dzrp` and `--debugger` are mutually exclusive.

## Debug IO space

Some IO addresses unused by the EZ80F92 are used by the emulator for debugging
purposes:

| IO addresses  | Function                                                           |
| ------------- | ------------------------------------------------------------------ |
| 0x00          | Terminate emulator (exit code will be the value written to IO 0x0) |
| 0x10-0x1f     | Breakpoint (requires --debugger)                                   |
| 0x20-0x2f     | Print CPU state (requires --debugger)                              |

These functions are activated by write (not read), and the upper 8-bits of the
IO address are ignored. ie:

```
	out (0),a
```

will shut down the emulator.

## Other command-line options

Read about other command-line options with:

```
fab-agon-emulator --help
agon-ez80 --help
agon-vdp-cli --help
```

## Reports & Documentation

Technical reports and design documents are in the [reports/](./reports/) directory:

- [Split VDP Architecture](./reports/2026-02-03-001-split-vdp-architecture.md) - Networked eZ80/VDP communication protocol

## Submodules

- `zinc/` - [ZINC Is Not CP/M](https://github.com/nihirash/ZINC) - CP/M compatibility layer for Agon

## Mac-specific issues

The Fab Agon Emulator executables provided on [releases](https://github.com/tomm/fab-agon-emulator/releases)
are not signed, so in order to run them on your Mac you need to run the following command from
the directory containing the fab-agon-emulator executable:

```
xattr -dr com.apple.quarantine fab-agon-emulator firmware/*.so
```
