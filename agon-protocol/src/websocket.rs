//! WebSocket support for eZ80/VDP communication.
//!
//! This module provides WebSocket server and connection handling that uses
//! the same message protocol as Unix/TCP sockets.

use std::net::{TcpListener, TcpStream};
use tungstenite::{accept, WebSocket};
use tungstenite::protocol::Message as WsMessage;

use crate::{Message, ProtocolError};

/// A WebSocket listener that accepts connections
pub struct WebSocketListener {
    listener: TcpListener,
    port: u16,
}

impl WebSocketListener {
    /// Bind to a TCP port and start listening for WebSocket connections
    pub fn bind(port: u16) -> Result<Self, std::io::Error> {
        let addr = format!("0.0.0.0:{}", port);
        let listener = TcpListener::bind(&addr)?;
        Ok(WebSocketListener { listener, port })
    }

    /// Accept a new WebSocket connection (blocking)
    ///
    /// This performs the WebSocket handshake automatically.
    pub fn accept(&self) -> Result<WebSocketConnection, std::io::Error> {
        let (stream, _addr) = self.listener.accept()?;
        // Disable Nagle's algorithm for lower latency
        let _ = stream.set_nodelay(true);

        // Perform WebSocket handshake
        let websocket = accept(stream).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::ConnectionRefused, e.to_string())
        })?;

        Ok(WebSocketConnection { websocket })
    }

    /// Set non-blocking mode on the listener
    pub fn set_nonblocking(&self, nonblocking: bool) -> Result<(), std::io::Error> {
        self.listener.set_nonblocking(nonblocking)
    }

    /// Get the port this listener is bound to
    pub fn port(&self) -> u16 {
        self.port
    }
}

/// A WebSocket connection for bidirectional message exchange
pub struct WebSocketConnection {
    websocket: WebSocket<TcpStream>,
}

impl WebSocketConnection {
    /// Send a protocol message over WebSocket
    pub fn send(&mut self, msg: &Message) -> Result<(), ProtocolError> {
        let data = msg.encode();
        self.websocket
            .send(WsMessage::Binary(data.into()))
            .map_err(|e| ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::BrokenPipe,
                e.to_string(),
            )))
    }

    /// Receive a protocol message from WebSocket (blocking)
    pub fn recv(&mut self) -> Result<Message, ProtocolError> {
        loop {
            let ws_msg = self.websocket.read().map_err(Self::convert_ws_error)?;

            match ws_msg {
                WsMessage::Binary(data) => {
                    let (msg, _len) = Message::decode(&data)?;
                    return Ok(msg);
                }
                WsMessage::Close(_) => {
                    return Err(ProtocolError::Io(std::io::Error::new(
                        std::io::ErrorKind::ConnectionReset,
                        "WebSocket closed",
                    )));
                }
                WsMessage::Ping(data) => {
                    // Respond to ping with pong
                    let _ = self.websocket.send(WsMessage::Pong(data));
                }
                WsMessage::Pong(_) => {
                    // Ignore pong messages
                }
                WsMessage::Text(_) => {
                    // Ignore text messages, we only use binary
                }
                WsMessage::Frame(_) => {
                    // Raw frames shouldn't appear in normal operation
                }
            }
        }
    }

    /// Convert tungstenite error to ProtocolError, preserving WouldBlock
    fn convert_ws_error(e: tungstenite::Error) -> ProtocolError {
        match e {
            tungstenite::Error::Io(io_err) => ProtocolError::Io(io_err),
            other => ProtocolError::Io(std::io::Error::new(
                std::io::ErrorKind::ConnectionReset,
                other.to_string(),
            )),
        }
    }

    /// Try to receive a message (non-blocking)
    /// Returns None if no message is available
    pub fn try_recv(&mut self) -> Result<Option<Message>, ProtocolError> {
        // Get the underlying stream and set non-blocking
        let stream = self.websocket.get_ref();
        stream.set_nonblocking(true).map_err(ProtocolError::Io)?;

        let result = match self.recv() {
            Ok(msg) => Ok(Some(msg)),
            Err(ProtocolError::Io(ref e)) if e.kind() == std::io::ErrorKind::WouldBlock => Ok(None),
            Err(e) => Err(e),
        };

        // Restore blocking mode
        let _ = self.websocket.get_ref().set_nonblocking(false);
        result
    }

    /// Close the WebSocket connection gracefully
    pub fn close(&mut self) -> Result<(), std::io::Error> {
        self.websocket.close(None).map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other, e.to_string())
        })?;
        // Flush pending close frame
        let _ = self.websocket.flush();
        Ok(())
    }

    /// Check if the connection is still open
    pub fn is_open(&self) -> bool {
        self.websocket.can_read() && self.websocket.can_write()
    }
}
