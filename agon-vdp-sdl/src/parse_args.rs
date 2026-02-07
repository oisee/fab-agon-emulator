//! Command-line argument parsing for agon-vdp-sdl.

use std::path::PathBuf;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    Quiet = 0,
    Verbose = 1,
    Trace = 2,
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
    -h, --help              Show this help

EXAMPLES:
    # Start with default settings (Unix socket)
    agon-vdp-sdl

    # Connect to remote eZ80 with Quark firmware
    agon-vdp-sdl --tcp 192.168.1.100:5000 --firmware quark

    # Start with custom VDP library
    agon-vdp-sdl --vdp ./my_vdp.so
"#
    );
}
