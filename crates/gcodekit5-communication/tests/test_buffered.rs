use gcodekit5_communication::{
    BufferedCommunicatorWrapper, BufferedCommunicatorConfig, Communicator, 
    CommunicatorListenerHandle, ConnectionParams
};
use std::sync::{Arc, Mutex};

// Mock communicator for testing
struct MockCommunicator {
    sent_data: Arc<Mutex<Vec<String>>>,
    connected: bool,
}

impl MockCommunicator {
    fn new() -> Self {
        Self {
            sent_data: Arc::new(Mutex::new(Vec::new())),
            connected: false,
        }
    }
}

impl Communicator for MockCommunicator {
    fn connect(&mut self, _params: &ConnectionParams) -> gcodekit5_core::Result<()> {
        self.connected = true;
        Ok(())
    }

    fn disconnect(&mut self) -> gcodekit5_core::Result<()> {
        self.connected = false;
        Ok(())
    }

    fn is_connected(&self) -> bool {
        self.connected
    }

    fn send(&mut self, data: &[u8]) -> gcodekit5_core::Result<usize> {
        let s = String::from_utf8_lossy(data).to_string();
        self.sent_data.lock().unwrap().push(s);
        Ok(data.len())
    }

    fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>> {
        Ok(vec![])
    }

    fn add_listener(&mut self, _listener: CommunicatorListenerHandle) {}
    fn remove_listener(&mut self, _listener: &CommunicatorListenerHandle) {}
    fn connection_params(&self) -> Option<&ConnectionParams> { None }
    fn set_connection_params(&mut self, _params: ConnectionParams) -> gcodekit5_core::Result<()> { Ok(()) }
}

#[test]
fn test_buffered_queueing() {
    let mock = Box::new(MockCommunicator::new());
    let config = BufferedCommunicatorConfig {
        buffer_size: 100,
        queue_size: 10,
        max_retries: 3,
        flow_control: true,
    };
    
    let wrapper = BufferedCommunicatorWrapper::new(mock, config);
    
    // Queue some commands
    wrapper.queue_command("G0 X0 Y0".to_string()).unwrap();
    wrapper.queue_command("G1 X10 Y10".to_string()).unwrap();
    
    assert_eq!(wrapper.queued_commands_count().unwrap(), 2);
}

#[test]
fn test_buffered_streaming() {
    let _mock = Box::new(MockCommunicator::new());
    // Keep a reference to the sent data to verify
    // But wait, I passed the box to the wrapper, so I can't access the original mock easily 
    // unless I share the state.
    
    let sent_data = Arc::new(Mutex::new(Vec::new()));
    
    struct SharedMockCommunicator {
        sent_data: Arc<Mutex<Vec<String>>>,
        _connected: bool,
    }
    
    impl Communicator for SharedMockCommunicator {
        fn connect(&mut self, _params: &ConnectionParams) -> gcodekit5_core::Result<()> {
            self._connected = true;
            Ok(())
        }
        fn disconnect(&mut self) -> gcodekit5_core::Result<()> {
            self._connected = false;
            Ok(())
        }
        fn is_connected(&self) -> bool { self._connected }
        fn send(&mut self, data: &[u8]) -> gcodekit5_core::Result<usize> {
            let s = String::from_utf8_lossy(data).to_string();
            self.sent_data.lock().unwrap().push(s);
            Ok(data.len())
        }
        fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>> { Ok(vec![]) }
        fn add_listener(&mut self, _listener: CommunicatorListenerHandle) {}
        fn remove_listener(&mut self, _listener: &CommunicatorListenerHandle) {}
        fn connection_params(&self) -> Option<&ConnectionParams> { None }
        fn set_connection_params(&mut self, _params: ConnectionParams) -> gcodekit5_core::Result<()> { Ok(()) }
    }

    let mock = Box::new(SharedMockCommunicator {
        sent_data: sent_data.clone(),
        _connected: true,
    });
    
    let config = BufferedCommunicatorConfig {
        buffer_size: 100,
        queue_size: 10,
        max_retries: 3,
        flow_control: true,
    };
    
    let mut wrapper = BufferedCommunicatorWrapper::new(mock, config);
    
    wrapper.queue_command("G0 X0 Y0".to_string()).unwrap();
    wrapper.queue_command("G1 X10 Y10".to_string()).unwrap();
    
    // Stream commands
    wrapper.stream_commands().unwrap();
    
    // Verify commands were sent
    let sent = sent_data.lock().unwrap();
    assert_eq!(sent.len(), 4); // 2 commands + 2 newlines (send_command sends data then newline)
    assert_eq!(sent[0], "G0 X0 Y0");
    assert_eq!(sent[1], "\n");
    assert_eq!(sent[2], "G1 X10 Y10");
    assert_eq!(sent[3], "\n");
    
    // Verify active commands
    assert_eq!(wrapper.active_commands_count().unwrap(), 2);
    assert_eq!(wrapper.queued_commands_count().unwrap(), 0);
}

#[test]
fn test_flow_control() {
    let sent_data = Arc::new(Mutex::new(Vec::new()));
    
    struct SharedMockCommunicator {
        sent_data: Arc<Mutex<Vec<String>>>,
        _connected: bool,
    }
    
    impl Communicator for SharedMockCommunicator {
        fn connect(&mut self, _params: &ConnectionParams) -> gcodekit5_core::Result<()> { Ok(()) }
        fn disconnect(&mut self) -> gcodekit5_core::Result<()> { Ok(()) }
        fn is_connected(&self) -> bool { true }
        fn send(&mut self, data: &[u8]) -> gcodekit5_core::Result<usize> {
            let s = String::from_utf8_lossy(data).to_string();
            self.sent_data.lock().unwrap().push(s);
            Ok(data.len())
        }
        fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>> { Ok(vec![]) }
        fn add_listener(&mut self, _listener: CommunicatorListenerHandle) {}
        fn remove_listener(&mut self, _listener: &CommunicatorListenerHandle) {}
        fn connection_params(&self) -> Option<&ConnectionParams> { None }
        fn set_connection_params(&mut self, _params: ConnectionParams) -> gcodekit5_core::Result<()> { Ok(()) }
    }

    let mock = Box::new(SharedMockCommunicator {
        sent_data: sent_data.clone(),
        _connected: true,
    });
    
    // Small buffer size to force flow control
    let config = BufferedCommunicatorConfig {
        buffer_size: 20, // Only enough for one command roughly
        queue_size: 10,
        max_retries: 3,
        flow_control: true,
    };
    
    let mut wrapper = BufferedCommunicatorWrapper::new(mock, config);
    
    // "G0 X0 Y0" is 8 chars + 1 newline = 9 bytes
    // "G1 X10 Y10" is 10 chars + 1 newline = 11 bytes
    // Total 20 bytes.
    
    wrapper.queue_command("G0 X0 Y0".to_string()).unwrap(); // 9 bytes
    wrapper.queue_command("G1 X10 Y10".to_string()).unwrap(); // 11 bytes. 9+11 = 20. Fits exactly?
    wrapper.queue_command("M5".to_string()).unwrap(); // 2+1 = 3 bytes. Should not fit.
    
    wrapper.stream_commands().unwrap();
    
    let sent = sent_data.lock().unwrap();
    // Should have sent first two commands
    // Wait, has_room_in_buffer check: used_space = sent_buffer_size + command_size + 1
    // 1. sent_buffer_size = 0. cmd="G0 X0 Y0" (8). used = 0 + 8 + 1 = 9 <= 20. OK.
    //    sent_buffer_size becomes 9.
    // 2. sent_buffer_size = 9. cmd="G1 X10 Y10" (10). used = 9 + 10 + 1 = 20 <= 20. OK.
    //    sent_buffer_size becomes 20.
    // 3. sent_buffer_size = 20. cmd="M5" (2). used = 20 + 2 + 1 = 23 > 20. Fail.
    
    // So we expect 4 sends (cmd, nl, cmd, nl)
    assert_eq!(sent.len(), 4);
    
    assert_eq!(wrapper.active_commands_count().unwrap(), 2);
    assert_eq!(wrapper.queued_commands_count().unwrap(), 1);
    
    drop(sent);
    
    // Acknowledge first command
    wrapper.handle_acknowledgment().unwrap();
    
    // Now buffer size should be 20 - 9 = 11.
    // Try streaming again
    wrapper.stream_commands().unwrap();
    
    // "M5" needs 3 bytes. 11 + 3 = 14 <= 20. OK.
    
    let sent = sent_data.lock().unwrap();
    assert_eq!(sent.len(), 6); // + cmd, nl
    assert_eq!(sent[4], "M5");
    
    assert_eq!(wrapper.active_commands_count().unwrap(), 2); // 1 remaining from before + 1 new
    assert_eq!(wrapper.queued_commands_count().unwrap(), 0);
}
