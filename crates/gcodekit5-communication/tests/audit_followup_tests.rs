//! Regression tests for the communication audit follow-ups: setting number width,
//! serial port filtering, buffered error handling, connection watch timing,
//! GRBL probe/work-offset handling and the g2core/TinyG placeholders.

use std::sync::Arc;
use std::time::Duration;

use gcodekit5_communication::communication::serial::is_valid_cnc_port_name;
use gcodekit5_communication::communication::{
    Communicator, CommunicatorListenerHandle, ConnectionParams,
};
use gcodekit5_communication::firmware::grbl::controller::{
    apply_status_positions, parse_active_wcs, parse_probe_report, parse_wcs_offset_report,
    wcs_offset_from_state, work_zero_command, GrblControllerState,
};
use gcodekit5_communication::firmware::grbl::utils::parse_setting_response;
use gcodekit5_communication::firmware::grbl::{GrblResponse, GrblResponseParser, StatusParser};
use gcodekit5_communication::firmware::{
    ConnectionWatchConfig, ConnectionWatchState, ConnectionWatcher, G2CoreController,
    TinyGController,
};
use gcodekit5_communication::{
    BufferedCommunicatorConfig, BufferedCommunicatorWrapper, CommandStatus,
};
use gcodekit5_core::ControllerTrait;
use parking_lot::Mutex;

// ---------- Item 4: setting numbers are u16 ----------

#[test]
fn test_settings_grblhal_extended_number_parses() {
    assert_eq!(
        parse_setting_response("$680=5"),
        Some((680, "5".to_string()))
    );
    let parser = GrblResponseParser::new();
    match parser.parse("$300=gcodekit") {
        Some(GrblResponse::Setting { number, value }) => {
            assert_eq!(number, 300);
            assert_eq!(value, "gcodekit");
        }
        other => panic!("expected setting, got {other:?}"),
    }
}

// ---------- Item 5: serial port filtering ----------

#[test]
fn test_serial_port_filter_includes_uarts_and_bluetooth() {
    for name in [
        "/dev/ttyUSB0",
        "/dev/ttyACM1",
        "/dev/ttyS0",
        "/dev/ttyAMA0",
        "/dev/rfcomm0",
        "/dev/ttyGRBL",
        "COM3",
        "/dev/cu.usbmodem1421",
        "/dev/cu.HC-05",
    ] {
        assert!(is_valid_cnc_port_name(name), "{name} should be listed");
    }
}

#[test]
fn test_serial_port_filter_excludes_non_devices() {
    for name in [
        "/dev/tty1",
        "/dev/tty",
        "/dev/console",
        "/dev/ptmx",
        "/dev/pts/0",
        "/dev/ttyS",
        "/dev/tty.usbmodem1421",
        "/dev/cu.Bluetooth-Incoming-Port",
        "/dev/cu.debug-console",
        "COM",
        "COMX",
    ] {
        assert!(!is_valid_cnc_port_name(name), "{name} should be hidden");
    }
}

// ---------- Item 8: buffered errors are reported, never retried ----------

#[derive(Clone, Default)]
struct RecordingCommunicator {
    sent: Arc<Mutex<Vec<String>>>,
}

impl Communicator for RecordingCommunicator {
    fn connect(&mut self, _params: &ConnectionParams) -> gcodekit5_core::Result<()> {
        Ok(())
    }
    fn disconnect(&mut self) -> gcodekit5_core::Result<()> {
        Ok(())
    }
    fn is_connected(&self) -> bool {
        true
    }
    fn send(&mut self, data: &[u8]) -> gcodekit5_core::Result<usize> {
        Ok(data.len())
    }
    fn send_command(&mut self, command: &str) -> gcodekit5_core::Result<()> {
        self.sent.lock().push(command.to_string());
        Ok(())
    }
    fn receive(&mut self) -> gcodekit5_core::Result<Vec<u8>> {
        Ok(Vec::new())
    }
    fn add_listener(&mut self, _listener: CommunicatorListenerHandle) {}
    fn remove_listener(&mut self, _listener: &CommunicatorListenerHandle) {}
    fn connection_params(&self) -> Option<&ConnectionParams> {
        None
    }
    fn set_connection_params(&mut self, _params: ConnectionParams) -> gcodekit5_core::Result<()> {
        Ok(())
    }
}

#[test]
fn test_buffered_error_is_reported_not_resent() {
    let comm = RecordingCommunicator::default();
    let sent = Arc::clone(&comm.sent);
    let mut wrapper =
        BufferedCommunicatorWrapper::new(Box::new(comm), BufferedCommunicatorConfig::default());
    for cmd in ["G1 X1", "G1 X2", "G1 X3"] {
        wrapper.queue_command(cmd.to_string()).expect("queue");
    }
    wrapper.stream_commands().expect("stream");
    assert_eq!(sent.lock().len(), 3);

    let failed = wrapper
        .handle_error("error:20".to_string())
        .expect("handle_error")
        .expect("a command was in flight");
    assert_eq!(failed.command, "G1 X1");
    assert_eq!(failed.status, CommandStatus::Failed);
    assert_eq!(failed.response.as_deref(), Some("error:20"));
    assert!(wrapper.is_paused(), "streaming must pause after an error");

    // Nothing is re-queued and nothing new goes out, even if streaming is driven again.
    wrapper.stream_commands().expect("stream");
    assert_eq!(wrapper.queued_commands_count().expect("count"), 0);
    assert_eq!(wrapper.active_commands_count().expect("count"), 2);
    assert_eq!(*sent.lock(), vec!["G1 X1", "G1 X2", "G1 X3"]);
}

// ---------- Item 7: connection watcher timing ----------

fn fast_watch_config() -> ConnectionWatchConfig {
    ConnectionWatchConfig {
        timeout_ms: 400,
        check_interval_ms: 10,
        enable_heartbeat: false,
        heartbeat_interval_ms: 1000,
    }
}

#[tokio::test]
async fn test_connection_watch_times_from_start_not_construction() {
    let watcher = ConnectionWatcher::new(fast_watch_config());
    // Longer than the timeout, before start(): must not count against the connection.
    tokio::time::sleep(Duration::from_millis(600)).await;
    watcher.start().await.expect("start");
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert_eq!(watcher.get_state().await, ConnectionWatchState::Healthy);
    assert!(watcher.time_since_heartbeat() < 400);

    tokio::time::sleep(Duration::from_millis(600)).await;
    assert_eq!(watcher.get_state().await, ConnectionWatchState::Lost);

    watcher.heartbeat();
    tokio::time::sleep(Duration::from_millis(40)).await;
    assert_eq!(watcher.get_state().await, ConnectionWatchState::Healthy);
    watcher.stop().await;
    assert!(!watcher.is_running().await);
}

#[tokio::test]
async fn test_connection_watch_restart_replaces_task() {
    let watcher = ConnectionWatcher::new(fast_watch_config());
    watcher.start().await.expect("start");
    tokio::time::sleep(Duration::from_millis(600)).await;
    assert!(watcher.is_lost().await);

    // A second start() resets the clock and replaces (does not leak) the old task.
    watcher.start().await.expect("restart");
    assert!(watcher.is_healthy().await);
    assert!(watcher.is_running().await);
    tokio::time::sleep(Duration::from_millis(50)).await;
    assert!(watcher.is_healthy().await);
    watcher.stop().await;
    assert!(!watcher.is_running().await);
}

// ---------- Item 10: GRBL probing and work offsets ----------

#[test]
fn test_grbl_probe_report_parsing() {
    let ok = parse_probe_report("[PRB:1.500,-2.250,-10.125:1]").expect("PRB");
    assert!(ok.success);
    assert_eq!(ok.position.x, 1.5);
    assert_eq!(ok.position.y, -2.25);
    assert_eq!(ok.position.z, -10.125);

    let miss = parse_probe_report("[PRB:0.000,0.000,-5.000:0]").expect("PRB");
    assert!(!miss.success);

    let four_axis = parse_probe_report("[PRB:1.0,2.0,3.0,45.0:1]").expect("PRB 4-axis");
    assert_eq!(four_axis.position.a, Some(45.0));

    assert!(parse_probe_report("[PRB:1.0,2.0]").is_none());
    assert!(parse_probe_report("[GC:G0 G54]").is_none());
    assert!(parse_probe_report("ok").is_none());
}

#[test]
fn test_grbl_wpos_derived_from_remembered_wco() {
    let mut state = GrblControllerState::default();
    // First report carries WCO.
    let first = StatusParser::parse_full(
        "<Idle|MPos:110.000,60.000,-5.000|FS:0,0|WCO:100.000,50.000,-20.000>",
    );
    apply_status_positions(&mut state, &first);
    assert_eq!(state.work_position.x, 10.0);
    assert_eq!(state.work_position.y, 10.0);
    assert_eq!(state.work_position.z, 15.0);

    // Later reports omit WCO; WPos must still be MPos - remembered WCO.
    let later = StatusParser::parse_full("<Run|MPos:120.000,55.000,-10.000|FS:500,0>");
    apply_status_positions(&mut state, &later);
    assert_eq!(state.work_position.x, 20.0);
    assert_eq!(state.work_position.y, 5.0);
    assert_eq!(state.work_position.z, 10.0);

    // $10 configured for WPos: derive MPos instead.
    let wpos_only = StatusParser::parse_full("<Idle|WPos:1.000,2.000,3.000|FS:0,0>");
    apply_status_positions(&mut state, &wpos_only);
    assert_eq!(state.machine_position.x, 101.0);
    assert_eq!(state.machine_position.y, 52.0);
    assert_eq!(state.machine_position.z, -17.0);

    // get_wcs_offset for the active WCS is the WCO, not the work position.
    let offset = wcs_offset_from_state(&state, 54).expect("offset");
    assert_eq!(offset.x, Some(100.0));
    assert_eq!(offset.y, Some(50.0));
    assert_eq!(offset.z, Some(-20.0));
    let current = wcs_offset_from_state(&state, 0).expect("offset");
    assert_eq!(current.x, Some(100.0));
}

#[test]
fn test_grbl_wcs_offsets_from_hash_report() {
    let mut state = GrblControllerState::default();
    assert!(
        wcs_offset_from_state(&state, 55).is_err(),
        "unknown offset must error"
    );

    let (wcs, off) = parse_wcs_offset_report("[G55:10.000,-5.000,2.500]").expect("G55");
    assert_eq!(wcs, 55);
    state.wcs_offsets.insert(wcs, off);
    let g55 = wcs_offset_from_state(&state, 55).expect("G55 offset");
    assert_eq!((g55.x, g55.y, g55.z), (Some(10.0), Some(-5.0), Some(2.5)));
    // P-number form (2 = G55) is accepted too.
    assert_eq!(wcs_offset_from_state(&state, 2).expect("P2").x, Some(10.0));
    assert!(wcs_offset_from_state(&state, 60).is_err());

    assert!(parse_wcs_offset_report("[G28:0.000,0.000,0.000]").is_none());
    assert!(parse_wcs_offset_report("[G92:0.000,0.000,0.000]").is_none());
    assert_eq!(
        parse_active_wcs("[GC:G0 G55 G17 G21 G90 G94 M5 M9 T0 F0 S0]"),
        Some(55)
    );
    assert_eq!(parse_active_wcs("[GC:G0 G17 G21]"), None);
}

#[test]
fn test_grbl_work_zero_uses_g10_l20() {
    assert_eq!(
        work_zero_command("XYZ").as_deref(),
        Some("G10 L20 P0 X0 Y0 Z0")
    );
    assert_eq!(work_zero_command("z").as_deref(), Some("G10 L20 P0 Z0"));
    assert_eq!(work_zero_command("Q"), None);
}

// ---------- Item 11: g2core / TinyG placeholders fail loudly ----------

#[tokio::test]
async fn test_g2core_tinyg_report_not_supported() {
    let mut g2 = G2CoreController::new(ConnectionParams::default(), None).expect("g2core");
    assert!(g2.connect().await.is_err());
    assert!(!g2.is_connected());
    let err = g2
        .send_command("G0 X1")
        .await
        .expect_err("must not succeed");
    assert!(err.to_string().contains("not supported"), "{err}");
    assert!(g2.jog_incremental('X', 1.0, 100.0).await.is_err());

    let mut tg = TinyGController::new(ConnectionParams::default(), None).expect("tinyg");
    assert!(tg.connect().await.is_err());
    let err = tg
        .send_command("G0 X1")
        .await
        .expect_err("must not succeed");
    assert!(err.to_string().contains("not supported"), "{err}");
    assert!(tg.jog_incremental('X', 1.0, 100.0).await.is_err());
    assert!(tg.probe_z(50.0).await.is_err());
}
