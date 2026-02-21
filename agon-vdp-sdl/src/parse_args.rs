//! Command-line argument parsing for agon-vdp-sdl.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet = 0,
    Verbose = 1,
    Trace = 2,
}

/// Specifies which frames to dump. Supports individual numbers and
/// inclusive ranges (e.g. `1,2,3,500,600..800`). Empty = dump all.
#[derive(Debug, Clone)]
pub struct FrameSpec {
    entries: Vec<FrameSpecEntry>,
}

#[derive(Debug, Clone)]
enum FrameSpecEntry {
    Single(u64),
    Range(u64, u64),
}

impl FrameSpec {
    pub fn all() -> Self {
        FrameSpec { entries: vec![] }
    }

    pub fn parse(spec: &str) -> Result<FrameSpec, String> {
        let spec = spec.trim();
        if spec.is_empty() {
            return Ok(Self::all());
        }
        let mut entries = Vec::new();
        for part in spec.split(',') {
            let part = part.trim();
            if part.is_empty() {
                continue;
            }
            if let Some((start_s, end_s)) = part.split_once("..") {
                let start: u64 = start_s.trim().parse()
                    .map_err(|_| format!("Invalid range start '{}' in '{}'", start_s.trim(), part))?;
                let end: u64 = end_s.trim().parse()
                    .map_err(|_| format!("Invalid range end '{}' in '{}'", end_s.trim(), part))?;
                if start > end {
                    return Err(format!("Invalid range: {} > {} in '{}'", start, end, part));
                }
                entries.push(FrameSpecEntry::Range(start, end));
            } else {
                let n: u64 = part.parse()
                    .map_err(|_| format!("Invalid frame number '{}'", part))?;
                entries.push(FrameSpecEntry::Single(n));
            }
        }
        Ok(FrameSpec { entries })
    }

    pub fn includes(&self, n: u64) -> bool {
        if self.entries.is_empty() {
            return true;
        }
        for entry in &self.entries {
            match entry {
                FrameSpecEntry::Single(v) => {
                    if *v == n { return true; }
                }
                FrameSpecEntry::Range(start, end) => {
                    if n >= *start && n <= *end { return true; }
                }
            }
        }
        false
    }
}

pub struct AppArgs {
    pub socket_path: Option<String>,
    pub tcp_addr: Option<String>,
    pub firmware: String,
    pub vdp_path: Option<PathBuf>,
    pub verbosity: Verbosity,
    pub fullscreen: bool,
    pub dump_frames: Option<String>,
    pub dump_keyframes: Option<String>,
    pub frame_spec: FrameSpec,
    pub replay: Option<PathBuf>,
    pub replay_raw: bool,
    pub replay_fps: Option<f64>,
    pub replay_log: Option<String>,
}

pub fn parse_args() -> Result<AppArgs, String> {
    let mut args = AppArgs {
        socket_path: None,
        tcp_addr: None,
        firmware: "console8".to_string(),
        vdp_path: None,
        verbosity: Verbosity::Quiet,
        fullscreen: false,
        dump_frames: None,
        dump_keyframes: None,
        frame_spec: FrameSpec::all(),
        replay: None,
        replay_raw: false,
        replay_fps: None,
        replay_log: None,
    };

    let mut argv: Vec<String> = std::env::args().collect();
    argv.remove(0); // program name

    while !argv.is_empty() {
        let arg = argv.remove(0);
        match arg.as_str() {
            "-h" | "--help" => {
                print_help();
                std::process::exit(0);
            }
            "-s" | "--socket" => {
                if argv.is_empty() {
                    return Err("--socket requires a path".to_string());
                }
                args.socket_path = Some(argv.remove(0));
            }
            "--tcp" => {
                if argv.is_empty() {
                    return Err("--tcp requires a host:port".to_string());
                }
                args.tcp_addr = Some(argv.remove(0));
            }
            "-f" | "--firmware" => {
                if argv.is_empty() {
                    return Err("--firmware requires a name".to_string());
                }
                args.firmware = argv.remove(0);
            }
            "--vdp" => {
                if argv.is_empty() {
                    return Err("--vdp requires a path".to_string());
                }
                args.vdp_path = Some(PathBuf::from(argv.remove(0)));
            }
            "-v" => {
                args.verbosity = Verbosity::Verbose;
            }
            "-vv" => {
                args.verbosity = Verbosity::Trace;
            }
            "--fullscreen" => {
                args.fullscreen = true;
            }
            "--dump-frames" => {
                if argv.is_empty() {
                    return Err("--dump-frames requires a directory path".to_string());
                }
                args.dump_frames = Some(argv.remove(0));
            }
            "--dump-keyframes" => {
                if argv.is_empty() {
                    return Err("--dump-keyframes requires a directory path".to_string());
                }
                args.dump_keyframes = Some(argv.remove(0));
            }
            s if s.starts_with("--frame-spec=") => {
                let spec = s.trim_start_matches("--frame-spec=");
                args.frame_spec = FrameSpec::parse(spec)?;
            }
            "--frame-spec" => {
                if argv.is_empty() {
                    return Err("--frame-spec requires a value (e.g. 1,2,3,600..800)".to_string());
                }
                args.frame_spec = FrameSpec::parse(&argv.remove(0))?;
            }
            "--replay" => {
                if argv.is_empty() {
                    return Err("--replay requires a file path".to_string());
                }
                args.replay = Some(PathBuf::from(argv.remove(0)));
            }
            "--replay-raw" => {
                args.replay_raw = true;
            }
            "--replay-fps" => {
                if argv.is_empty() {
                    return Err("--replay-fps requires a number".to_string());
                }
                let val: f64 = argv.remove(0).parse()
                    .map_err(|_| "--replay-fps requires a valid number".to_string())?;
                args.replay_fps = Some(val);
            }
            "--replay-log" => {
                if argv.is_empty() {
                    return Err("--replay-log requires a file path (or '-' for stderr)".to_string());
                }
                args.replay_log = Some(argv.remove(0));
            }
            other => {
                return Err(format!("Unknown argument: {}", other));
            }
        }
    }

    Ok(args)
}

fn print_help() {
    eprintln!(
        r#"agon-vdp-sdl - Graphical VDP client for Agon emulator

Connects to a running agon-ez80 instance.

USAGE:
    agon-vdp-sdl [OPTIONS]

OPTIONS:
    -s, --socket <path>     Unix socket path (default: /tmp/agon-vdp.sock)
    --tcp <host:port>       Connect via TCP instead of Unix socket
    -f, --firmware <name>   VDP firmware: console8, quark, electron (default: console8)
    --vdp <path>            Explicit path to VDP .so library
    -v                      Verbose output
    -vv                     Trace output (more verbose)
    --fullscreen            Start in fullscreen mode
    --dump-frames <dir>     Save every frame as PNG on each vsync
    --dump-keyframes <dir>  Save frame only when UART data arrived since last vsync
    --frame-spec <spec>     Only dump specific frames (e.g. 1,2,3,500,600..800)
    --replay <file>         Replay VDU bytes from file instead of connecting
    --replay-raw            Treat replay file as raw bytes (no chunk framing)
    --replay-fps <N>        Override VSYNC rate for replay (default: 60, 0=max speed)
    --replay-log <file>     Log replay events to file ('-' for stderr)
    -h, --help              Show this help

EXAMPLES:
    # Start with default settings (Unix socket)
    agon-vdp-sdl

    # Connect to remote eZ80 with Quark firmware
    agon-vdp-sdl --tcp 192.168.1.100:5000 --firmware quark

    # Start with custom VDP library
    agon-vdp-sdl --vdp ./my_vdp.so

    # Replay a VDU stream and dump specific frames
    agon-vdp-sdl --replay stream.vdu --dump-frames ./frames --frame-spec 1,100..200

    # Quick parse-check of a VDU stream
    agon-vdp-sdl --replay stream.vdu --replay-fps 0 --replay-log -
"#
    );
}
