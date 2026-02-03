//! Socket abstraction for Unix sockets and TCP connections.

use std::io::{BufReader, BufWriter, Read, Write};
use std::net::{TcpListener, TcpStream};
#[cfg(unix)]
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::Path;
use std::time::Duration;

use crate::{Message, ProtocolError};

/// Default socket path for Unix sockets
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/agon-vdp.sock";

/// Socket address type - either Unix socket path or TCP address
#[derive(Debug, Clone)]
pub enum SocketAddr {
    #[cfg(unix)]
    Unix(String),
    Tcp(String),
}

impl SocketAddr {
    /// Create a Unix socket address
    #[cfg(unix)]
    pub fn unix<P: AsRef<Path>>(path: P) -> Self {
        SocketAddr::Unix(path.as_ref().to_string_lossy().to_string())
    }

    /// Create a TCP socket address
    pub fn tcp<S: Into<String>>(addr: S) -> Self {
        SocketAddr::Tcp(addr.into())
    }
}

impl std::fmt::Display for SocketAddr {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            #[cfg(unix)]
            SocketAddr::Unix(path) => write!(f, "{}", path),
            SocketAddr::Tcp(addr) => write!(f, "{}", addr),
        }
    }
}

/// Internal enum for listener types
enum ListenerInner {
    #[cfg(unix)]
    Unix(UnixListener),
    Tcp(TcpListener),
}

/// A socket listener that accepts connections
pub struct SocketListener {
    inner: ListenerInner,
    addr: SocketAddr,
}

impl SocketListener {
    /// Bind to a socket address and start listening
    pub fn bind(addr: &SocketAddr) -> Result<Self, std::io::Error> {
        match addr {
            #[cfg(unix)]
            SocketAddr::Unix(path) => {
                // Remove existing socket file if present
                let _ = std::fs::remove_file(path);
                let listener = UnixListener::bind(path)?;
                Ok(SocketListener {
                    inner: ListenerInner::Unix(listener),
                    addr: addr.clone(),
                })
            }
            SocketAddr::Tcp(addr_str) => {
                let listener = TcpListener::bind(addr_str)?;
                Ok(SocketListener {
                    inner: ListenerInner::Tcp(listener),
                    addr: addr.clone(),
                })
            }
        }
    }

    /// Accept a new connection (blocking)
    pub fn accept(&self) -> Result<SocketConnection, std::io::Error> {
        match &self.inner {
            #[cfg(unix)]
            ListenerInner::Unix(listener) => {
                let (stream, _) = listener.accept()?;
                Ok(SocketConnection::from_unix(stream))
            }
            ListenerInner::Tcp(listener) => {
                let (stream, _) = listener.accept()?;
                Ok(SocketConnection::from_tcp(stream))
            }
        }
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), std::io::Error> {
        match &self.inner {
            #[cfg(unix)]
            ListenerInner::Unix(listener) => listener.set_nonblocking(nonblocking),
            ListenerInner::Tcp(listener) => listener.set_nonblocking(nonblocking),
        }
    }

    /// Get the address this listener is bound to
    pub fn addr(&self) -> &SocketAddr {
        &self.addr
    }
}

#[cfg(unix)]
impl Drop for SocketListener {
    fn drop(&mut self) {
        // Clean up Unix socket file on drop
        if let SocketAddr::Unix(path) = &self.addr {
            let _ = std::fs::remove_file(path);
        }
    }
}

/// Internal enum for connection stream types
enum StreamInner {
    #[cfg(unix)]
    Unix(UnixStream),
    Tcp(TcpStream),
}

impl StreamInner {
    fn try_clone(&self) -> Result<Self, std::io::Error> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => Ok(StreamInner::Unix(s.try_clone()?)),
            StreamInner::Tcp(s) => Ok(StreamInner::Tcp(s.try_clone()?)),
        }
    }

    fn set_nonblocking(&self, nonblocking: bool) -> Result<(), std::io::Error> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => s.set_nonblocking(nonblocking),
            StreamInner::Tcp(s) => s.set_nonblocking(nonblocking),
        }
    }

    fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), std::io::Error> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => s.set_read_timeout(dur),
            StreamInner::Tcp(s) => s.set_read_timeout(dur),
        }
    }

    fn set_write_timeout(&self, dur: Option<Duration>) -> Result<(), std::io::Error> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => s.set_write_timeout(dur),
            StreamInner::Tcp(s) => s.set_write_timeout(dur),
        }
    }

    fn shutdown(&self, how: std::net::Shutdown) -> Result<(), std::io::Error> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => s.shutdown(how),
            StreamInner::Tcp(s) => s.shutdown(how),
        }
    }
}

impl Read for StreamInner {
    fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => s.read(buf),
            StreamInner::Tcp(s) => s.read(buf),
        }
    }
}

impl Write for StreamInner {
    fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => s.write(buf),
            StreamInner::Tcp(s) => s.write(buf),
        }
    }

    fn flush(&mut self) -> std::io::Result<()> {
        match self {
            #[cfg(unix)]
            StreamInner::Unix(s) => s.flush(),
            StreamInner::Tcp(s) => s.flush(),
        }
    }
}

/// A connection to a remote socket
pub struct SocketConnection {
    reader: BufReader<StreamInner>,
    writer: BufWriter<StreamInner>,
}

impl SocketConnection {
    #[cfg(unix)]
    fn from_unix(stream: UnixStream) -> Self {
        let reader = BufReader::new(StreamInner::Unix(stream.try_clone().unwrap()));
        let writer = BufWriter::new(StreamInner::Unix(stream));
        SocketConnection { reader, writer }
    }

    fn from_tcp(stream: TcpStream) -> Self {
        // Disable Nagle's algorithm for lower latency
        let _ = stream.set_nodelay(true);
        let reader = BufReader::new(StreamInner::Tcp(stream.try_clone().unwrap()));
        let writer = BufWriter::new(StreamInner::Tcp(stream));
        SocketConnection { reader, writer }
    }

    /// Connect to a socket address
    pub fn connect(addr: &SocketAddr) -> Result<Self, std::io::Error> {
        match addr {
            #[cfg(unix)]
            SocketAddr::Unix(path) => {
                let stream = UnixStream::connect(path)?;
                Ok(Self::from_unix(stream))
            }
            SocketAddr::Tcp(addr_str) => {
                let stream = TcpStream::connect(addr_str)?;
                Ok(Self::from_tcp(stream))
            }
        }
    }

    /// Connect with timeout
    pub fn connect_timeout(addr: &SocketAddr, timeout: Duration) -> Result<Self, std::io::Error> {
        match addr {
            #[cfg(unix)]
            SocketAddr::Unix(path) => {
                // Unix sockets don't have a built-in connect_timeout, use blocking connect
                let stream = UnixStream::connect(path)?;
                Ok(Self::from_unix(stream))
            }
            SocketAddr::Tcp(addr_str) => {
                let socket_addr: std::net::SocketAddr = addr_str
                    .parse()
                    .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidInput, e))?;
                let stream = TcpStream::connect_timeout(&socket_addr, timeout)?;
                Ok(Self::from_tcp(stream))
            }
        }
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), std::io::Error> {
        self.reader.get_ref().set_nonblocking(nonblocking)?;
        self.writer.get_ref().set_nonblocking(nonblocking)?;
        Ok(())
    }

    /// Set read timeout
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), std::io::Error> {
        self.reader.get_ref().set_read_timeout(dur)
    }

    /// Set write timeout
    pub fn set_write_timeout(&self, dur: Option<Duration>) -> Result<(), std::io::Error> {
        self.writer.get_ref().set_write_timeout(dur)
    }

    /// Send a message
    pub fn send(&mut self, msg: &Message) -> Result<(), ProtocolError> {
        msg.write_to(&mut self.writer)
    }

    /// Receive a message (blocking)
    pub fn recv(&mut self) -> Result<Message, ProtocolError> {
        Message::read_from(&mut self.reader)
    }

    /// Try to receive a message (non-blocking)
    /// Returns None if no message is available
    pub fn try_recv(&mut self) -> Result<Option<Message>, ProtocolError> {
        // Set to non-blocking temporarily
        self.reader
            .get_ref()
            .set_nonblocking(true)
            .map_err(ProtocolError::Io)?;

        let result = match Message::read_from(&mut self.reader) {
            Ok(msg) => Ok(Some(msg)),
            Err(ProtocolError::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        };

        // Restore blocking mode
        let _ = self.reader.get_ref().set_nonblocking(false);
        result
    }

    /// Clone the connection (creates separate reader/writer that share the underlying socket)
    pub fn try_clone(&self) -> Result<Self, std::io::Error> {
        let reader = BufReader::new(self.reader.get_ref().try_clone()?);
        let writer = BufWriter::new(self.writer.get_ref().try_clone()?);
        Ok(SocketConnection { reader, writer })
    }

    /// Shutdown the connection
    pub fn shutdown(&self) -> Result<(), std::io::Error> {
        self.writer.get_ref().shutdown(std::net::Shutdown::Both)
    }

    /// Split into separate reader and writer halves
    pub fn split(self) -> (SocketReader, SocketWriter) {
        (
            SocketReader {
                reader: self.reader,
            },
            SocketWriter {
                writer: self.writer,
            },
        )
    }
}

/// Reader half of a split connection
pub struct SocketReader {
    reader: BufReader<StreamInner>,
}

impl SocketReader {
    /// Receive a message (blocking)
    pub fn recv(&mut self) -> Result<Message, ProtocolError> {
        Message::read_from(&mut self.reader)
    }

    /// Set read timeout
    pub fn set_read_timeout(&self, dur: Option<Duration>) -> Result<(), std::io::Error> {
        self.reader.get_ref().set_read_timeout(dur)
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), std::io::Error> {
        self.reader.get_ref().set_nonblocking(nonblocking)
    }
}

/// Writer half of a split connection
pub struct SocketWriter {
    writer: BufWriter<StreamInner>,
}

impl SocketWriter {
    /// Send a message
    pub fn send(&mut self, msg: &Message) -> Result<(), ProtocolError> {
        msg.write_to(&mut self.writer)
    }

    /// Set write timeout
    pub fn set_write_timeout(&self, dur: Option<Duration>) -> Result<(), std::io::Error> {
        self.writer.get_ref().set_write_timeout(dur)
    }

    /// Set non-blocking mode
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), std::io::Error> {
        self.writer.get_ref().set_nonblocking(nonblocking)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;

    #[test]
    #[cfg(unix)]
    fn test_unix_socket_communication() {
        let socket_path = "/tmp/agon-test-socket.sock";
        let addr = SocketAddr::unix(socket_path);

        // Start server in background thread
        let addr_clone = addr.clone();
        let server_thread = thread::spawn(move || {
            let listener = SocketListener::bind(&addr_clone).unwrap();
            let mut conn = listener.accept().unwrap();

            // Receive hello
            let msg = conn.recv().unwrap();
            assert!(matches!(msg, Message::Hello { version: 1, .. }));

            // Send ack
            conn.send(&Message::HelloAck {
                version: 1,
                capabilities: "{}".to_string(),
            })
            .unwrap();

            // Receive some data
            let msg = conn.recv().unwrap();
            assert_eq!(msg, Message::UartData(vec![0x41, 0x42]));

            // Send data back
            conn.send(&Message::UartData(vec![0x43, 0x44])).unwrap();
        });

        // Give server time to start
        thread::sleep(Duration::from_millis(50));

        // Connect as client
        let mut conn = SocketConnection::connect(&addr).unwrap();

        // Send hello
        conn.send(&Message::Hello {
            version: 1,
            flags: 0,
        })
        .unwrap();

        // Receive ack
        let msg = conn.recv().unwrap();
        assert!(matches!(msg, Message::HelloAck { version: 1, .. }));

        // Send data
        conn.send(&Message::UartData(vec![0x41, 0x42])).unwrap();

        // Receive data
        let msg = conn.recv().unwrap();
        assert_eq!(msg, Message::UartData(vec![0x43, 0x44]));

        server_thread.join().unwrap();
    }
}
