const HELP: &str = "\
Agon VDP CLI - Text-only VDP client

Connects to a running agon-ez80 instance.

USAGE:
  agon-vdp-cli [OPTIONS]

OPTIONS:
  -h, --help            Prints help information
  --socket <path>       Unix socket path (default: /tmp/agon-vdp.sock)
  --tcp <host:port>     Connect via TCP instead of Unix socket
  -v, --verbose         Show connection and protocol events
  -vv, --trace          Show all protocol messages
  -vvv, --trace-uart    Show individual UART bytes (very verbose)
  --log <file>          Write trace output to file instead of stderr
";

/// Verbosity level for debug output
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Verbosity {
    /// No debug output
    Quiet = 0,
    /// Connection events, errors
    Verbose = 1,
    /// All protocol messages (VSYNC, CTS, etc.)
    Trace = 2,
    /// Individual UART bytes
    TraceUart = 3,
}

impl Default for Verbosity {
    fn default() -> Self {
        Verbosity::Quiet
    }
}

#[derive(Debug)]
pub struct AppArgs {
    pub socket_path: Option<String>,
    pub tcp_addr: Option<String>,
    pub verbosity: Verbosity,
    pub log_file: Option<String>,
}

pub fn parse_args() -> Result<AppArgs, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    // Count -v flags for verbosity level
    let verbosity = if pargs.contains("--trace-uart") || pargs.contains("-vvv") {
        Verbosity::TraceUart
    } else if pargs.contains("--trace") || pargs.contains("-vv") {
        Verbosity::Trace
    } else if pargs.contains(["-v", "--verbose"]) {
        Verbosity::Verbose
    } else {
        Verbosity::Quiet
    };

    let args = AppArgs {
        socket_path: pargs.opt_value_from_str("--socket")?,
        tcp_addr: pargs.opt_value_from_str("--tcp")?,
        verbosity,
        log_file: pargs.opt_value_from_str("--log")?,
    };

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(args)
}
