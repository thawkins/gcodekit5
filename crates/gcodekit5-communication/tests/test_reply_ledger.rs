//! Tests for matching controller replies to the lines they answer.

use gcodekit5_communication::communication::reply_ledger::ReplyLedger;

#[test]
fn test_reply_ledger_matches_replies_in_order() {
    let mut ledger = ReplyLedger::new();
    // `$I` then a tagged `$13=-1`: the first reply answers `$I`.
    ledger.on_lines_sent(1);
    ledger.on_lines_sent(1);
    assert!(ledger.tag_last(7));
    assert_eq!(ledger.on_reply(), None);
    assert_eq!(ledger.on_reply(), Some(7));
    assert!(ledger.is_empty());
}

#[test]
fn test_reply_ledger_reply_without_outstanding_line_is_untagged() {
    let mut ledger = ReplyLedger::new();
    assert_eq!(ledger.on_reply(), None);
    assert!(!ledger.tag_last(1));
}

#[test]
fn test_reply_ledger_clear_forgets_tags() {
    let mut ledger = ReplyLedger::new();
    ledger.on_lines_sent(3);
    ledger.tag_last(9);
    ledger.clear();
    ledger.on_lines_sent(1);
    assert_eq!(ledger.on_reply(), None);
    assert_eq!(ledger.len(), 0);
}

#[test]
fn test_reply_ledger_overflow_clears_instead_of_misaligning() {
    let mut ledger = ReplyLedger::new();
    ledger.on_lines_sent(4096);
    ledger.on_lines_sent(1);
    assert!(ledger.is_empty());
}

/// A port that accepts every write and never has data to read.
struct SinkPort;

impl gcodekit5_communication::communication::serial::SerialPort for SinkPort {
    fn write(&mut self, data: &[u8]) -> std::io::Result<usize> {
        Ok(data.len())
    }
    fn read(&mut self, _buf: &mut [u8]) -> std::io::Result<usize> {
        Ok(0)
    }
    fn name(&self) -> String {
        "sink".to_string()
    }
    fn close(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}

#[test]
fn test_reply_ledger_serial_communicator_tags_the_right_line() {
    use gcodekit5_communication::{Communicator, SerialCommunicator};

    let mut comm = SerialCommunicator::with_port(Box::new(SinkPort));
    comm.send_command("$I").expect("send");
    // Realtime bytes have no newline and get no reply.
    comm.send(b"?").expect("send");
    comm.send_tagged_command("$13=-1", 42).expect("send");

    assert_eq!(comm.take_reply_tag(), None, "the first reply answers $I");
    assert_eq!(comm.take_reply_tag(), Some(42));
    assert_eq!(comm.take_reply_tag(), None);
}

#[test]
fn test_reply_ledger_serial_communicator_clear_drops_pending_tags() {
    use gcodekit5_communication::SerialCommunicator;

    let mut comm = SerialCommunicator::with_port(Box::new(SinkPort));
    comm.send_tagged_command("$13=1", 5).expect("send");
    comm.clear_reply_ledger();
    assert_eq!(comm.take_reply_tag(), None);
}

#[test]
fn test_reply_ledger_unread_data_is_received_first() {
    use gcodekit5_communication::{Communicator, SerialCommunicator};

    let mut comm = SerialCommunicator::with_port(Box::new(SinkPort));
    comm.unread(b"ok\nerr");
    comm.unread(b"or:20\n");
    assert_eq!(comm.receive().expect("receive"), b"ok\nerror:20\n".to_vec());
    // Then the port again (the sink has nothing).
    assert!(comm.receive().expect("receive").is_empty());
}
