//! Tests for TCP connections through `SerialCommunicator` (network-attached controllers).

use std::io::{Read, Write};
use std::net::TcpListener;
use std::time::{Duration, Instant};

use gcodekit5_communication::{Communicator, ConnectionParams, SerialCommunicator};

/// Reads until data arrives or `timeout` elapses.
fn receive_within(comm: &mut SerialCommunicator, timeout: Duration) -> Vec<u8> {
    let deadline = Instant::now() + timeout;
    loop {
        let data = comm
            .receive()
            .expect("receive should not fail while connected");
        if !data.is_empty() || Instant::now() >= deadline {
            return data;
        }
    }
}

#[test]
fn test_tcp_transport_sends_and_receives() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local listener");
    let port = listener.local_addr().expect("listener address").port();

    let server = std::thread::spawn(move || {
        let (mut stream, _) = listener.accept().expect("accept connection");
        let mut buf = [0u8; 16];
        let n = stream.read(&mut buf).expect("read from client");
        assert_eq!(&buf[..n], b"?");
        stream
            .write_all(b"<Idle|MPos:0,0,0>\r\n")
            .expect("reply to client");
        std::thread::sleep(Duration::from_millis(200));
    });

    let mut comm = SerialCommunicator::new();
    comm.connect(&ConnectionParams::tcp("127.0.0.1", port))
        .expect("TCP connect should be supported");
    assert!(comm.is_connected());
    comm.send(b"?").expect("send over TCP");
    let reply = receive_within(&mut comm, Duration::from_secs(2));
    assert!(
        String::from_utf8_lossy(&reply).starts_with("<Idle"),
        "{reply:?}"
    );
    server.join().expect("server thread");
}

#[test]
fn test_tcp_transport_reports_closed_connection() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local listener");
    let port = listener.local_addr().expect("listener address").port();
    let server = std::thread::spawn(move || {
        let (stream, _) = listener.accept().expect("accept connection");
        drop(stream); // controller goes away
    });

    let mut comm = SerialCommunicator::new();
    comm.connect(&ConnectionParams::tcp("127.0.0.1", port))
        .expect("TCP connect should be supported");
    server.join().expect("server thread");

    let deadline = Instant::now() + Duration::from_secs(2);
    let mut saw_error = false;
    while Instant::now() < deadline {
        if comm.receive().is_err() {
            saw_error = true;
            break;
        }
    }
    assert!(
        saw_error,
        "a closed TCP connection must be reported, not read as no data"
    );
    assert!(
        !comm.is_connected(),
        "a closed TCP connection must no longer report as connected"
    );
    assert!(
        comm.receive().is_err(),
        "receive after the peer closed must keep failing, not read as no data"
    );
}

#[test]
fn test_tcp_transport_idle_receive_returns_quickly() {
    let listener = TcpListener::bind("127.0.0.1:0").expect("bind local listener");
    let port = listener.local_addr().expect("listener address").port();
    let (done_tx, done_rx) = std::sync::mpsc::channel::<()>();
    let server = std::thread::spawn(move || {
        // Accept and stay silent until the test is done, like an idle controller.
        let (_stream, _) = listener.accept().expect("accept connection");
        let _ = done_rx.recv_timeout(Duration::from_secs(10));
    });

    let params = ConnectionParams::tcp("127.0.0.1", port);
    assert!(
        params.timeout_ms >= 1000,
        "test assumes the default (long) connection timeout, got {} ms",
        params.timeout_ms
    );
    let mut comm = SerialCommunicator::new();
    comm.connect(&params)
        .expect("TCP connect should be supported");

    let started = Instant::now();
    let data = comm
        .receive()
        .expect("an idle receive should report no data, not an error");
    let elapsed = started.elapsed();

    assert!(data.is_empty(), "{data:?}");
    assert!(
        elapsed < Duration::from_millis(200),
        "idle TCP receive blocked for {elapsed:?}; it holds the communicator lock that \
         Pause and E-stop need"
    );
    assert!(comm.is_connected());

    comm.disconnect().expect("disconnect");
    done_tx.send(()).expect("signal server");
    server.join().expect("server thread");
}

#[test]
fn test_websocket_connect_is_rejected_clearly() {
    let mut comm = SerialCommunicator::new();
    assert!(
        comm.connect(&ConnectionParams::websocket("127.0.0.1", 81))
            .is_err()
    );
    assert!(!comm.is_connected());
}

/// A port whose cable has been pulled: every read and write fails the way
/// serialport reports a hang-up.
struct HungUpPort;

impl gcodekit5_communication::communication::serial::SerialPort for HungUpPort {
    fn write(&mut self, _data: &[u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Err(std::io::ErrorKind::BrokenPipe.into())
    }
    fn name(&self) -> String {
        "hung-up".to_string()
    }
    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_serial_hangup_on_read_disconnects() {
    // Audit B-01: BrokenPipe (USB unplug) was read as "no data" forever.
    let mut comm = SerialCommunicator::with_port(Box::new(HungUpPort));
    assert!(comm.is_connected());
    assert!(comm.receive().is_err());
    assert!(!comm.is_connected());
}

#[test]
fn test_serial_hangup_on_write_disconnects() {
    let mut comm = SerialCommunicator::with_port(Box::new(HungUpPort));
    assert!(comm.send(b"?").is_err());
    assert!(!comm.is_connected());
}
