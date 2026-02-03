const HELP: &str = "\
Agon eZ80 - Standalone eZ80 emulator

Connects to an external VDP server via socket.

USAGE:
  agon-ez80 [OPTIONS]

OPTIONS:
  -h, --help            Prints help information
  --socket <path>       Unix socket path (default: /tmp/agon-vdp.sock)
  --tcp <host:port>     Use TCP instead of Unix socket
  --mos <path>          Use a different MOS.bin firmware
  --sdcard-img <file>   Use a raw SDCard image rather than the host filesystem
  --sdcard <path>       Sets the path of the emulated SDCard
  -u, --unlimited-cpu   Don't limit eZ80 CPU frequency
  -z, --zero            Initialize RAM with zeroes instead of random values
  -d, --debugger        Enable debugger
  -b, --breakpoint <addr>  Set initial breakpoint (hex address)
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
    pub sdcard: Option<String>,
    pub sdcard_img: Option<String>,
    pub unlimited_cpu: bool,
    pub zero: bool,
    pub mos_bin: Option<std::path::PathBuf>,
    pub debugger: bool,
    pub breakpoints: Vec<u32>,
    pub verbosity: Verbosity,
    pub log_file: Option<String>,
}

pub fn parse_args() -> Result<AppArgs, pico_args::Error> {
    let mut pargs = pico_args::Arguments::from_env();

    // for `make install`
    if pargs.contains("--prefix") {
        print!("{}", option_env!("PREFIX").unwrap_or(""));
        std::process::exit(0);
    }

    if pargs.contains(["-h", "--help"]) {
        print!("{}", HELP);
        std::process::exit(0);
    }

    let breakpoints: Vec<u32> = pargs
        .values_from_fn(["-b", "--breakpoint"], |s| {
            u32::from_str_radix(s.trim_start_matches("0x"), 16)
        })?;

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
        sdcard: pargs.opt_value_from_str("--sdcard")?,
        sdcard_img: pargs.opt_value_from_str("--sdcard-img")?,
        unlimited_cpu: pargs.contains(["-u", "--unlimited-cpu"]),
        zero: pargs.contains(["-z", "--zero"]),
        mos_bin: pargs.opt_value_from_str("--mos")?,
        debugger: pargs.contains(["-d", "--debugger"]),
        breakpoints,
        verbosity,
        log_file: pargs.opt_value_from_str("--log")?,
    };

    let remaining = pargs.finish();
    if !remaining.is_empty() {
        eprintln!("Warning: unused arguments left: {:?}.", remaining);
    }

    Ok(args)
}
