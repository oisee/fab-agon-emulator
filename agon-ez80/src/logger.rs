//! Simple logger that can write to stderr or a file.

use crate::parse_args::Verbosity;
use std::fs::File;
use std::io::{self, BufWriter, Write};
use std::sync::{Arc, Mutex};

/// Output destination for logger
enum Output {
    Stderr,
    File(BufWriter<File>),
}

/// Thread-safe logger
pub struct Logger {
    output: Arc<Mutex<Output>>,
    verbosity: Verbosity,
}

impl Logger {
    /// Create a new logger writing to stderr
    pub fn stderr(verbosity: Verbosity) -> Self {
        Logger {
            output: Arc::new(Mutex::new(Output::Stderr)),
            verbosity,
        }
    }

    /// Create a new logger writing to a file
    pub fn file(path: &str, verbosity: Verbosity) -> io::Result<Self> {
        let file = File::create(path)?;
        Ok(Logger {
            output: Arc::new(Mutex::new(Output::File(BufWriter::new(file)))),
            verbosity,
        })
    }

    /// Get verbosity level
    pub fn verbosity(&self) -> Verbosity {
        self.verbosity
    }

    /// Log a message if verbosity level is met
    pub fn log(&self, level: Verbosity, msg: &str) {
        if self.verbosity >= level {
            if let Ok(mut output) = self.output.lock() {
                match &mut *output {
                    Output::Stderr => {
                        eprintln!("{}", msg);
                    }
                    Output::File(f) => {
                        let _ = writeln!(f, "{}", msg);
                        let _ = f.flush();
                    }
                }
            }
        }
    }

    /// Log at Verbose level
    pub fn verbose(&self, msg: &str) {
        self.log(Verbosity::Verbose, msg);
    }

    /// Log at Trace level
    pub fn trace(&self, msg: &str) {
        self.log(Verbosity::Trace, msg);
    }

    /// Log at TraceUart level
    pub fn trace_uart(&self, msg: &str) {
        self.log(Verbosity::TraceUart, msg);
    }

    /// Always log (for errors, important info)
    pub fn info(&self, msg: &str) {
        if let Ok(mut output) = self.output.lock() {
            match &mut *output {
                Output::Stderr => {
                    eprintln!("{}", msg);
                }
                Output::File(f) => {
                    let _ = writeln!(f, "{}", msg);
                    let _ = f.flush();
                }
            }
        }
    }
}

impl Clone for Logger {
    fn clone(&self) -> Self {
        Logger {
            output: self.output.clone(),
            verbosity: self.verbosity,
        }
    }
}
