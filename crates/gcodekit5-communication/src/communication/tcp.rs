//! TCP/Network communication implementation
//!
//! Provides TCP/IP network communication for remote CNC controller connections.
//!
//! Supports:
//! - TCP client connections
//! - Hostname/IP and port configuration
//! - Connection timeout
//! - Read/write operations
//! - Connection pooling preparation
//! - Error handling and recovery

use crate::{ConnectionDriver, ConnectionParams};
use gcodekit5_core::{Error, Result};
use std::io::{self, Read, Write};
use std::net::{TcpStream, ToSocketAddrs};
use std::time::Duration;

/// TCP connection information
#[derive(Debug, Clone)]
pub struct TcpConnectionInfo {
    /// Remote host address (hostname or IP)
    pub host: String,

    /// Remote port number
    pub port: u16,

    /// Local bind address (optional)
    pub local_addr: Option<String>,

    /// Connection timeout
    pub timeout: Duration,
}

impl TcpConnectionInfo {
    /// Create new TCP connection info
    pub fn new(host: &str, port: u16) -> Self {
        Self {
            host: host.to_string(),
            port,
            local_addr: None,
            timeout: Duration::from_secs(5),
        }
    }

    /// Set the connection timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the local bind address
    pub fn with_local_addr(mut self, addr: &str) -> Self {
        self.local_addr = Some(addr.to_string());
        self
    }

    /// Get connection address string (host:port)
    pub fn address(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }
}

/// TCP network interface trait
pub trait TcpPort: Send + Sync {
    /// Write data to the connection
    fn write(&mut self, data: &[u8]) -> io::Result<usize>;

    /// Read data from the connection
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize>;

    /// Get the peer address string
    fn peer_addr(&self) -> io::Result<String>;

    /// Close the connection
    fn close(&mut self) -> io::Result<()>;
}

/// Real TCP connection using std::net::TcpStream
pub struct RealTcpPort {
    stream: TcpStream,
}

impl RealTcpPort {
    /// Open a TCP connection with the given parameters
    pub fn open(params: &ConnectionParams) -> Result<Self> {
        if params.driver != ConnectionDriver::Tcp {
            return Err(Error::other("RealTcpPort requires Tcp driver type"));
        }

        let address = format!("{}:{}", params.port, params.network_port);

        // Resolve via ToSocketAddrs so hostnames (e.g. "fluidnc.local") and IPv6
        // literals work; `SocketAddr::parse` only accepts numeric "ip:port".
        let addrs: Vec<_> = (params.port.as_str(), params.network_port)
            .to_socket_addrs()
            .map_err(|e| Error::other(format!("Invalid address '{}': {}", address, e)))?
            .collect();
        if addrs.is_empty() {
            return Err(Error::other(format!(
                "Address '{}' did not resolve",
                address
            )));
        }

        // A hostname may resolve to several addresses (e.g. IPv6 and IPv4); try each in turn
        // and only fail once all of them have been tried.
        let timeout = Duration::from_millis(params.timeout_ms);
        let mut last_err = None;
        for socket_addr in &addrs {
            match TcpStream::connect_timeout(socket_addr, timeout) {
                Ok(stream) => {
                    stream
                        .set_read_timeout(Some(timeout))
                        .map_err(|e| Error::other(format!("Failed to set read timeout: {}", e)))?;
                    stream
                        .set_write_timeout(Some(timeout))
                        .map_err(|e| Error::other(format!("Failed to set write timeout: {}", e)))?;
                    return Ok(RealTcpPort { stream });
                }
                Err(e) => {
                    tracing::debug!("TCP connect to {} failed: {}", socket_addr, e);
                    last_err = Some(e);
                }
            }
        }

        let e = last_err.map_or_else(|| "no addresses".to_string(), |e| e.to_string());
        tracing::error!("Failed to connect to TCP server {}: {}", address, e);
        Err(Error::other(format!(
            "Failed to connect to {}: {}",
            address, e
        )))
    }

    /// Sets the read timeout of the underlying stream.
    ///
    /// [`RealTcpPort::open`] uses `params.timeout_ms` for connecting, reading and
    /// writing. Callers that poll for incoming data (such as `SerialCommunicator`)
    /// should shorten the read timeout so an idle read returns quickly; an idle read
    /// then fails with `WouldBlock` (Unix) or `TimedOut` (Windows).
    ///
    /// # Arguments
    /// * `timeout` - The new read timeout; `None` blocks until data arrives.
    ///
    /// # Errors
    /// Returns an error if `timeout` is `Some(Duration::ZERO)` or the OS rejects it.
    pub fn set_read_timeout(&self, timeout: Option<Duration>) -> io::Result<()> {
        self.stream.set_read_timeout(timeout)
    }

    /// Reconnect the TCP connection
    pub fn reconnect(&mut self, params: &ConnectionParams) -> Result<()> {
        self.close().ok();
        let new_port = RealTcpPort::open(params)?;
        self.stream = new_port.stream;
        Ok(())
    }
}

impl TcpPort for RealTcpPort {
    fn write(&mut self, data: &[u8]) -> io::Result<usize> {
        // write_all: a short write would silently drop the tail of a G-code line
        self.stream.write_all(data).map(|()| data.len())
    }

    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        self.stream.read(buf)
    }

    fn peer_addr(&self) -> io::Result<String> {
        Ok(self.stream.peer_addr()?.to_string())
    }

    fn close(&mut self) -> io::Result<()> {
        let _ = self.stream.shutdown(std::net::Shutdown::Both);
        Ok(())
    }
}
