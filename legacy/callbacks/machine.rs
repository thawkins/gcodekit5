use std::rc::Rc;
use std::sync::{Arc, Mutex, atomic::{AtomicBool, Ordering}};
use slint::{ComponentHandle, VecModel};
use crate::slint_generatedMainWindow::MainWindow;
use crate::ErrorDialog;
use gcodekit5::{SerialCommunicator, ConnectionParams, ConnectionDriver, SerialParity};
use gcodekit5::{DeviceConsoleManager as ConsoleManager, DeviceMessageType, ConsoleListener, CapabilityManager, Communicator};
use crate::app::types::GcodeSendState;
use crate::app::helpers::{update_device_info_panel, sync_capabilities_to_ui, get_available_ports};
use gcodekit5_communication::firmware::grbl::error_decoder::format_error;
use tracing::warn;

pub fn register_callbacks(
    main_window: &MainWindow,
    communicator: Arc<Mutex<SerialCommunicator>>,
    console_manager: Arc<ConsoleManager>,
    ports_model: Rc<VecModel<slint::SharedString>>,
    status_polling_stop: Arc<AtomicBool>,
    status_polling_active: Arc<AtomicBool>,
    capability_manager: Rc<CapabilityManager>,
    gcode_send_state: Arc<Mutex<GcodeSendState>>,
    detected_firmware: Arc<Mutex<Option<gcodekit5::firmware::firmware_detector::FirmwareDetectionResult>>>,
) {
    // Set up refresh-ports callback
    let ports_model_clone = ports_model.clone();
    let window_weak = main_window.as_weak();
    main_window.on_refresh_ports(move || {
        if let Ok(ports) = get_available_ports() {
            ports_model_clone.set_vec(ports.clone());
            
            if let Some(window) = window_weak.upgrade() {
                let current_selection = window.get_selected_port();
                // If no port is selected or the placeholder is selected, select the first available port
                if !ports.is_empty() && (current_selection.is_empty() || current_selection == "No ports available") {
                    window.set_selected_port(ports[0].clone());
                }
            }
        }
    });

    // Set up connect callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let polling_stop_connect = status_polling_stop.clone();
    let capability_manager_clone = capability_manager.clone();
    let gcode_send_state_connect = gcode_send_state.clone();
    let detected_firmware_connect = detected_firmware.clone();
    main_window.on_connect(move |port: slint::SharedString, baud: i32| {
        let port_str = port.to_string();

        // Add connection attempt to console
        console_manager_clone.add_message(
            DeviceMessageType::Output,
            format!("Connecting to {} at {} baud", port_str, baud),
        );

        // Update UI with connecting status immediately
        if let Some(window) = window_weak.upgrade() {
            window.set_connection_status(slint::SharedString::from("Connecting..."));
            window.set_device_version(slint::SharedString::from("Detecting..."));
            window.set_machine_state(slint::SharedString::from("CONNECTING"));
            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }

        // Create connection parameters
        let params = ConnectionParams {
            driver: ConnectionDriver::Serial,
            port: port_str.clone(),
            network_port: 8888,
            baud_rate: baud as u32,
            timeout_ms: 50,
            flow_control: false,
            data_bits: 8,
            stop_bits: 1,
            parity: SerialParity::None,
            auto_reconnect: true,
            max_retries: 3,
        };

        // Try to connect
        let mut comm = communicator_clone.lock().unwrap();
        match comm.connect(&params) {
            Ok(()) => {
                console_manager_clone.add_message(
                    DeviceMessageType::Success,
                    format!("Successfully connected to {} at {} baud", port_str, baud),
                );

                if let Some(window) = window_weak.upgrade() {
                    window.set_connected(true);
                    window.set_connection_status(slint::SharedString::from("Connected"));
                    window.set_device_version(slint::SharedString::from("GRBL 1.1"));
                    window.set_machine_state(slint::SharedString::from("IDLE"));
                    let console_output = console_manager_clone.get_output();
                    window.set_console_output(slint::SharedString::from(console_output));

                    // Initialize Device Info panel with default GRBL 1.1
                    // Will be updated after firmware detection completes
                    use gcodekit5::firmware::firmware_version::{FirmwareType, SemanticVersion};
                    let firmware_type = FirmwareType::Grbl;
                    let version = SemanticVersion::new(1, 1, 0);
                    update_device_info_panel(&window, firmware_type, version, &capability_manager_clone);

                    // Set up timer to check for firmware detection and update Device Info
                    let window_weak_timer = window_weak.clone();
                    let detected_firmware_timer = detected_firmware_connect.clone();
                    let capability_manager_timer = capability_manager_clone.clone();
                    let timer = slint::Timer::default();
                    timer.start(slint::TimerMode::Repeated, std::time::Duration::from_millis(500), move || {
                        if let Some(detection) = detected_firmware_timer.lock().unwrap().as_ref().cloned() {
                            if let Some(window) = window_weak_timer.upgrade() {
                                update_device_info_panel(&window, detection.firmware_type, detection.version, &capability_manager_timer);
                                window.set_device_version(slint::SharedString::from(
                                    format!("{} {}", detection.firmware_type, detection.version)
                                ));
                            }
                            // Stop timer after updating once
                        }
                    });
                }

                // Start status polling thread
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    "Starting status polling...".to_string(),
                );

                polling_stop_connect.store(false, Ordering::Relaxed);
                let window_weak_poll = window_weak.clone();
                let polling_active = status_polling_active.clone();
                let polling_stop = polling_stop_connect.clone();
                let communicator_poll = communicator_clone.clone();
                let console_manager_poll = console_manager_clone.clone();
                let gcode_state_poll = gcode_send_state_connect.clone();

                std::thread::spawn(move || {
                    polling_active.store(true, Ordering::Relaxed);

                    // Send $I once at startup to get firmware version
                    {
                        let mut comm = communicator_poll.lock().unwrap();
                        if let Err(e) = comm.send_command("$I") {
                            warn!("Failed to send $I for firmware detection: {}", e);
                        } else {
                        }
                    }

                    // Wait for firmware detection to complete (listener will process the response)
                    // The UI timer will update Device Info panel automatically
                    std::thread::sleep(std::time::Duration::from_millis(1000));

                    // GRBL buffer is 128 bytes, but we use 127 for safety
                    const GRBL_RX_BUFFER_SIZE: usize = 127;
                    let mut response_buffer = String::new();

                    // Main polling loop runs at 35ms intervals
                    // - Reads responses continuously (ok, error, status reports)
                    // - Sends G-code lines using character-counting protocol
                    // - Sends ? status query every 35ms (real-time command)
                    while !polling_stop.load(Ordering::Relaxed) {
                        std::thread::sleep(std::time::Duration::from_millis(35));

                        // Use the shared communicator instead of creating a new connection
                        // CRITICAL: Hold the lock for minimal time to allow jog commands through
                        let (response_data, is_connected) = {
                            let mut comm = communicator_poll.lock().unwrap();
                            let connected = comm.is_connected();
                            let response = if connected { comm.receive().ok() } else { None };
                            (response, connected)
                        }; // Lock released immediately after reading

                        if is_connected {
                            // Step 1: Process responses (without holding lock)
                            if let Some(response) = response_data {
                                if !response.is_empty() {
                                    response_buffer.push_str(&String::from_utf8_lossy(&response));
                                    
                                    // Process complete lines
                                    while let Some(idx) = response_buffer.find('\n') {
                                        let line = response_buffer[..idx].trim().to_string();
                                        response_buffer.drain(..idx + 1);
                                        
                                        if line.is_empty() { continue; }

                                        // Count "ok" and "error" responses for buffer management
                                        let is_ok = line.contains("ok") || line.contains("OK");
                                        let is_error = line.contains("error:");
                                        
                                        if is_ok || is_error {
                                            let mut gstate = gcode_state_poll.lock().unwrap();
                                            if let Some(len) = gstate.line_lengths.pop_front() {
                                                gstate.pending_bytes = gstate.pending_bytes.saturating_sub(len);
                                            }

                                            if let Some(sent_cmd) = gstate.sent_lines.pop_front() {
                                                let log_msg = format!("{} => {}", sent_cmd, line);
                                                // Release lock before logging to avoid potential deadlocks (though unlikely here)
                                                drop(gstate);
                                                console_manager_poll.add_message(
                                                    DeviceMessageType::Output,
                                                    log_msg
                                                );
                                            } else {
                                                drop(gstate);
                                            }
                                        }

                                        // Check for errors and handle them
                                        if is_error {
                                            // Decode error code if present
                                            let error_msg = if let Some(code_str) = line.strip_prefix("error:") {
                                                if let Ok(code) = code_str.trim().parse::<u8>() {
                                                    format_error(code)
                                                } else {
                                                    format!("GRBL error: {}", line)
                                                }
                                            } else {
                                                format!("GRBL error: {}", line)
                                            };
                                            
                                            console_manager_poll.add_message(
                                                DeviceMessageType::Error,
                                                error_msg.clone()
                                            );

                                            // Show error dialog
                                            let wh = window_weak_poll.clone();
                                            let em = error_msg.clone();
                                            slint::invoke_from_event_loop(move || {
                                                if let Some(_w) = wh.upgrade() {
                                                    let error_dialog = ErrorDialog::new().unwrap();
                                                    error_dialog.set_error_message(slint::SharedString::from(format!(
                                                        "GRBL Error\n\nThe device reported an error.\n\n{}",
                                                        em
                                                    )));

                                                    let error_dialog_weak = error_dialog.as_weak();
                                                    error_dialog.on_close_dialog(move || {
                                                        if let Some(dlg) = error_dialog_weak.upgrade() {
                                                            dlg.hide().ok();
                                                        }
                                                    });

                                                    error_dialog.show().ok();
                                                }
                                            }).ok();
                                        }

                                        // Process status responses
                                        if line.contains("<") && line.contains(">") {
                                            // Parse full status from response
                                            use gcodekit5::firmware::grbl::status_parser::StatusParser;
                                            let full_status = StatusParser::parse_full(&line);

                                            let window_handle = window_weak_poll.clone();
                                            let raw_response = line.clone();
                                            slint::invoke_from_event_loop(move || {
                                                if let Some(window) = window_handle.upgrade() {
                                                    // Update raw status response
                                                    window.set_raw_status_response(slint::SharedString::from(raw_response.trim()));

                                                    // Update machine position
                                                    if let Some(mpos) = full_status.mpos {
                                                        window.set_position_x(mpos.x as f32);
                                                        window.set_position_y(mpos.y as f32);
                                                        window.set_position_z(mpos.z as f32);
                                                        if let Some(a) = mpos.a {
                                                            window.set_position_a(a as f32);
                                                        }
                                                        if let Some(b) = mpos.b {
                                                            window.set_position_b(b as f32);
                                                        }
                                                        if let Some(c) = mpos.c {
                                                            window.set_position_c(c as f32);
                                                        }
                                                    }

                                                    // Update work position
                                                    if let Some(wpos) = full_status.wpos {
                                                        window.set_work_position_x(wpos.x as f32);
                                                        window.set_work_position_y(wpos.y as f32);
                                                        window.set_work_position_z(wpos.z as f32);
                                                    } else if let (Some(mpos), Some(wco)) = (full_status.mpos, full_status.wco) {
                                                        // Calculate WPos if not provided directly
                                                        window.set_work_position_x((mpos.x - wco.x) as f32);
                                                        window.set_work_position_y((mpos.y - wco.y) as f32);
                                                        window.set_work_position_z((mpos.z - wco.z) as f32);
                                                    } else if let Some(mpos) = full_status.mpos {
                                                        // Fallback to MPos if WPos/WCO not available (e.g. no offset)
                                                        window.set_work_position_x(mpos.x as f32);
                                                        window.set_work_position_y(mpos.y as f32);
                                                        window.set_work_position_z(mpos.z as f32);
                                                    }

                                                    // Update machine state
                                                    if let Some(state) = full_status.machine_state {
                                                        window.set_machine_state(slint::SharedString::from(state));
                                                    }

                                                    // Update feed rate
                                                    if let Some(feed) = full_status.feed_rate {
                                                        window.set_feed_rate(feed as f32);
                                                    }

                                                    // Update spindle speed
                                                    if let Some(spindle) = full_status.spindle_speed {
                                                        window.set_spindle_speed(spindle as f32);
                                                    }
                                                }
                                            })
                                            .ok();
                                        }
                                    }
                                }
                            }

                            // Step 2: Send G-code lines if queued
                            // Send up to 10 lines per cycle, respecting buffer limits
                            {
                                let mut gstate = gcode_state_poll.lock().unwrap();
                                let mut lines_sent_this_cycle = 0;

                                while !gstate.lines.is_empty() && lines_sent_this_cycle < 10 {
                                    let line = gstate.lines.front().cloned().unwrap_or_default();
                                    let trimmed = line.trim();

                                    // Skip empty lines and comments quickly
                                    if trimmed.is_empty() || trimmed.starts_with(';') {
                                        gstate.lines.pop_front();
                                        continue; // Don't count skipped lines
                                    }

                                    // Check buffer space before sending
                                    let line_len = trimmed.len() + 1;
                                    if gstate.pending_bytes + line_len <= GRBL_RX_BUFFER_SIZE {
                                        // Acquire lock only for the actual send operation
                                        let send_result = {
                                            let mut comm = communicator_poll.lock().unwrap();
                                            comm.send(format!("{}\n", trimmed).as_bytes())
                                        }; // Lock released immediately

                                        match send_result {
                                            Ok(_) => {
                                                gstate.lines.pop_front();
                                                gstate.pending_bytes += line_len;
                                                gstate.line_lengths.push_back(line_len);
                                                gstate.sent_lines.push_back(trimmed.to_string());
                                                gstate.total_sent += 1;
                                                lines_sent_this_cycle += 1;


                                                if gstate.total_sent.is_multiple_of(10) || gstate.lines.is_empty() {
                                                    let sent = gstate.total_sent;
                                                    let total = gstate.total_lines;
                                                    let progress = if total > 0 { (sent as f32 / total as f32) * 100.0 } else { 0.0 };
                                                    
                                                    // Calculate times
                                                    let (elapsed_str, estimated_str) = if let Some(start) = gstate.start_time {
                                                        let elapsed = start.elapsed();
                                                        let elapsed_secs = elapsed.as_secs();
                                                        let elapsed_formatted = format!("{:02}:{:02}:{:02}", elapsed_secs / 3600, (elapsed_secs % 3600) / 60, elapsed_secs % 60);
                                                        
                                                        let estimated_formatted = if progress > 0.0 {
                                                            let total_secs = (elapsed_secs as f32 / (progress / 100.0)) as u64;
                                                            let remaining_secs = total_secs.saturating_sub(elapsed_secs);
                                                            format!("{:02}:{:02}:{:02}", remaining_secs / 3600, (remaining_secs % 3600) / 60, remaining_secs % 60)
                                                        } else {
                                                            "--:--:--".to_string()
                                                        };
                                                        
                                                        (elapsed_formatted, estimated_formatted)
                                                    } else {
                                                        ("--:--:--".to_string(), "--:--:--".to_string())
                                                    };

                                                    let wh = window_weak_poll.clone();
                                                    slint::invoke_from_event_loop(move || {
                                                        if let Some(w) = wh.upgrade() {
                                                            w.set_connection_status(slint::SharedString::from(
                                                                format!("Sending: {}/{}", sent, total)
                                                            ));
                                                            w.set_progress_value(progress);
                                                            w.set_job_elapsed_time(slint::SharedString::from(elapsed_str));
                                                            w.set_job_estimated_time(slint::SharedString::from(estimated_str));
                                                        }
                                                    }).ok();
                                                }
                                            }
                                            Err(e) => {
                                                let error_msg = format!("✗ Send failed at line {}: {}", gstate.total_sent + 1, e);
                                                console_manager_poll.add_message(
                                                    DeviceMessageType::Error,
                                                    error_msg.clone()
                                                );
                                                gstate.lines.clear();

                                                // Show error dialog
                                                let wh = window_weak_poll.clone();
                                                let em = error_msg.clone();
                                                slint::invoke_from_event_loop(move || {
                                                    if let Some(_w) = wh.upgrade() {
                                                        let error_dialog = ErrorDialog::new().unwrap();
                                                        error_dialog.set_error_message(slint::SharedString::from(format!(
                                                            "Send Error\n\nFailed to send G-code to device.\n\n{}",
                                                            em
                                                        )));

                                                        let error_dialog_weak = error_dialog.as_weak();
                                                        error_dialog.on_close_dialog(move || {
                                                            if let Some(dlg) = error_dialog_weak.upgrade() {
                                                                dlg.hide().ok();
                                                            }
                                                        });

                                                        error_dialog.show().ok();
                                                    }
                                                }).ok();

                                                break;
                                            }
                                        }
                                    } else {
                                        break; // Buffer full, stop sending this cycle
                                    }
                                }

                                // Check if done sending
                                if gstate.total_lines > 0 && gstate.lines.is_empty() && gstate.line_lengths.is_empty() {
                                    let total = gstate.total_sent;
                                    console_manager_poll.add_message(
                                        DeviceMessageType::Success,
                                        format!("✓ Successfully sent {} lines", total)
                                    );
                                    let wh = window_weak_poll.clone();
                                    let cm = console_manager_poll.clone();
                                    slint::invoke_from_event_loop(move || {
                                        if let Some(w) = wh.upgrade() {
                                            w.set_connection_status(slint::SharedString::from(format!("Sent: {} lines", total)));
                                            w.set_progress_value(0.0);
                                            w.set_console_output(slint::SharedString::from(cm.get_output()));
                                        }
                                    }).ok();
                                    gstate.total_lines = 0;
                                    gstate.total_sent = 0;
                                }
                            } // Release gstate lock

                            // Step 3: Send status query periodically (real-time command - doesn't use buffer)
                            // Send every 200ms (4 cycles of 50ms)
                            static mut CYCLE: u32 = 0;
                            unsafe {
                                CYCLE += 1;
                                if CYCLE.is_multiple_of(4) {
                                    // Real-time command - acquire lock briefly for send only
                                    let mut comm = communicator_poll.lock().unwrap();
                                    comm.send(b"?").ok();
                                } // Lock released immediately
                            }
                        }
                    }
                    polling_active.store(false, Ordering::Relaxed);
                });
            }
            Err(e) => {
                let error_msg = format!("{}", e);
                console_manager_clone.add_message(
                    DeviceMessageType::Error,
                    format!("Connection failed: {}", error_msg),
                );
                if let Some(window) = window_weak.upgrade() {
                    window.set_connected(false);
                    window.set_connection_status(slint::SharedString::from("Connection Failed".to_string()));
                    window.set_device_version(slint::SharedString::from("Not Connected"));
                    window.set_machine_state(slint::SharedString::from("DISCONNECTED"));
                    let console_output = console_manager_clone.get_output();
                    window.set_console_output(slint::SharedString::from(console_output));

                    // Show error dialog
                    let error_dialog = ErrorDialog::new().unwrap();
                    error_dialog.set_error_message(slint::SharedString::from(format!(
                        "Connection Failed\n\nUnable to connect to {} at {} baud.\n\nError: {}",
                        port_str, baud, error_msg
                    )));

                    let error_dialog_weak = error_dialog.as_weak();
                    error_dialog.on_close_dialog(move || {
                        if let Some(dlg) = error_dialog_weak.upgrade() {
                            dlg.hide().ok();
                        }
                    });

                    error_dialog.show().ok();
                }
            }
        }
    });

    // Set up disconnect callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let polling_stop_clone = status_polling_stop.clone();
    let capability_manager_disconnect = capability_manager.clone();
    let detected_firmware_disconnect = detected_firmware.clone();
    main_window.on_disconnect(move || {
        console_manager_clone.add_message(DeviceMessageType::Output, "Disconnecting from device");

        // Stop the polling thread
        polling_stop_clone.store(true, Ordering::Relaxed);

        let mut comm = communicator_clone.lock().unwrap();
        match comm.disconnect() {
            Ok(()) => {
                // Reset the communicator to a fresh state by replacing with a new instance
                drop(comm);
                let mut new_comm = SerialCommunicator::new();
                // Re-register the console listener with the new communicator
                let console_listener = ConsoleListener::new_with_firmware_state(
                    console_manager_clone.clone(),
                    detected_firmware_disconnect.clone(),
                );
                new_comm.add_listener(console_listener);
                *communicator_clone.lock().unwrap() = new_comm;

                console_manager_clone
                    .add_message(DeviceMessageType::Success, "Successfully disconnected");
                if let Some(window) = window_weak.upgrade() {
                    window.set_connected(false);
                    window.set_connection_status(slint::SharedString::from("Disconnected"));
                    window.set_device_version(slint::SharedString::from("Not Connected"));
                    window.set_machine_state(slint::SharedString::from("DISCONNECTED"));
                    window.set_position_x(0.0);
                    window.set_position_y(0.0);
                    window.set_position_z(0.0);
                    let console_output = console_manager_clone.get_output();
                    window.set_console_output(slint::SharedString::from(console_output));

                    // Reset capabilities to defaults
                    capability_manager_disconnect.reset();
                    sync_capabilities_to_ui(&window, &capability_manager_disconnect);
                }
            }
            Err(e) => {
                console_manager_clone
                    .add_message(DeviceMessageType::Error, format!("Disconnect error: {}", e));
                if let Some(window) = window_weak.upgrade() {
                    window.set_connection_status(slint::SharedString::from("Disconnect error"));
                    let console_output = console_manager_clone.get_output();
                    window.set_console_output(slint::SharedString::from(console_output));

                    // Show error dialog
                    let error_dialog = ErrorDialog::new().unwrap();
                    error_dialog.set_error_message(slint::SharedString::from(format!(
                        "Disconnect Error\n\nAn error occurred while disconnecting from the device.\n\nError: {}",
                        e
                    )));

                    let error_dialog_weak = error_dialog.as_weak();
                    error_dialog.on_close_dialog(move || {
                        if let Some(dlg) = error_dialog_weak.upgrade() {
                            dlg.hide().ok();
                        }
                    });

                    error_dialog.show().ok();
                }
            }
        }
    });

    // Set up menu-view-machine callback
    let window_weak = main_window.as_weak();
    main_window.on_menu_view_machine(move || {
        if let Some(window) = window_weak.upgrade() {
            window.set_connection_status(slint::SharedString::from("Machine Control activated"));
        }
    });

    // Set up machine-zero-all callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    main_window.on_machine_zero_all(move || {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Zero All failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    "Zeroing all axes (G92 X0 Y0 Z0)...",
                );

                match comm.send_command("G92 X0 Y0 Z0") {
                    Ok(_) => {
                         console_manager_clone.add_message(
                            DeviceMessageType::Success,
                            "✓ Zero All command sent",
                        );
                    }
                    Err(e) => {
                        warn!("Failed to send Zero All command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Zero All failed: {}", e),
                        );
                    }
                }
            }
            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-emergency-stop callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    main_window.on_machine_emergency_stop(move || {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Emergency Stop failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                console_manager_clone.add_message(
                    DeviceMessageType::Error,
                    "!!! EMERGENCY STOP TRIGGERED !!!",
                );
                
                // Send Soft Reset (Ctrl-X / 0x18)
                match comm.send(&[0x18]) {
                    Ok(_) => {
                         console_manager_clone.add_message(
                            DeviceMessageType::Success,
                            "✓ Soft Reset sent",
                        );
                    }
                    Err(e) => {
                        warn!("Failed to send Emergency Stop: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Emergency Stop failed: {}", e),
                        );
                    }
                }
            }
            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-jog-home callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let gcode_send_state_home = gcode_send_state.clone();
    main_window.on_machine_jog_home(move || {
        if let Some(window) = window_weak.upgrade() {
            // Check if device is connected
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Jog Home failed: Device not connected");
                console_manager_clone.add_message(
                    DeviceMessageType::Error,
                    "✗ Device not connected. Please connect before sending commands.",
                );
                window.set_connection_status(slint::SharedString::from(
                    "Error: Device not connected",
                ));
            } else {
                // Send the Home command ($H)
                console_manager_clone
                    .add_message(DeviceMessageType::Output, "Sending Home command...");

                match comm.send_command("$H") {
                    Ok(_) => {
                        // Add to sent_lines for logging
                        if let Ok(mut gstate) = gcode_send_state_home.lock() {
                            gstate.sent_lines.push_back("$H".to_string());
                        }
                        
                        console_manager_clone.add_message(
                            DeviceMessageType::Success,
                            "✓ Home command sent to device",
                        );
                        window.set_connection_status(slint::SharedString::from("Homing..."));
                    }
                    Err(e) => {
                        warn!("Failed to send Home command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Failed to send Home command: {}", e),
                        );
                        window.set_connection_status(slint::SharedString::from(format!(
                            "Error sending Home command: {}",
                            e
                        )));
                    }
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-jog-x-positive callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let gcode_send_state_xp = gcode_send_state.clone();
    main_window.on_machine_jog_x_positive(move |step_size: f32| {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Jog X+ failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                // Send jog command in relative mode (G91) for incremental movement
                let jog_cmd = format!("$J=G91 X{} F2000", step_size);
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("Jogging X+ ({} mm)...", step_size),
                );

                match comm.send(format!("{}\n", jog_cmd).as_bytes()) {
                    Ok(_) => {
                        // Add to sent_lines for logging
                        if let Ok(mut gstate) = gcode_send_state_xp.lock() {
                            gstate.sent_lines.push_back(jog_cmd);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to send Jog X+ command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Jog X+ failed: {}", e),
                        );
                    }
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-jog-x-negative callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let gcode_send_state_xn = gcode_send_state.clone();
    main_window.on_machine_jog_x_negative(move |step_size: f32| {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Jog X- failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                // Send jog command in relative mode (G91) for incremental movement
                let jog_cmd = format!("$J=G91 X-{} F2000", step_size);
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("Jogging X- ({} mm)...", step_size),
                );

                match comm.send(format!("{}\n", jog_cmd).as_bytes()) {
                    Ok(_) => {
                        // Add to sent_lines for logging
                        if let Ok(mut gstate) = gcode_send_state_xn.lock() {
                            gstate.sent_lines.push_back(jog_cmd);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to send Jog X- command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Jog X- failed: {}", e),
                        );
                    }
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-jog-y-positive callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let gcode_send_state_yp = gcode_send_state.clone();
    main_window.on_machine_jog_y_positive(move |step_size: f32| {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Jog Y+ failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                // Send jog command in relative mode (G91) for incremental movement
                let jog_cmd = format!("$J=G91 Y{} F2000", step_size);
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("Jogging Y+ ({} mm)...", step_size),
                );

                match comm.send(format!("{}\n", jog_cmd).as_bytes()) {
                    Ok(_) => {
                        // Add to sent_lines for logging
                        if let Ok(mut gstate) = gcode_send_state_yp.lock() {
                            gstate.sent_lines.push_back(jog_cmd);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to send Jog Y+ command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Jog Y+ failed: {}", e),
                        );
                    }
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-jog-y-negative callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let gcode_send_state_yn = gcode_send_state.clone();
    main_window.on_machine_jog_y_negative(move |step_size: f32| {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Jog Y- failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                // Send jog command in relative mode (G91) for incremental movement
                let jog_cmd = format!("$J=G91 Y-{} F2000", step_size);
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("Jogging Y- ({} mm)...", step_size),
                );

                match comm.send(format!("{}\n", jog_cmd).as_bytes()) {
                    Ok(_) => {
                        // Add to sent_lines for logging
                        if let Ok(mut gstate) = gcode_send_state_yn.lock() {
                            gstate.sent_lines.push_back(jog_cmd);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to send Jog Y- command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Jog Y- failed: {}", e),
                        );
                    }
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-jog-z-positive callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let gcode_send_state_zp = gcode_send_state.clone();
    main_window.on_machine_jog_z_positive(move |step_size: f32| {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Jog Z+ failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                // Send jog command in relative mode (G91) for incremental movement
                let jog_cmd = format!("$J=G91 Z{} F2000", step_size);
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("Jogging Z+ ({} mm)...", step_size),
                );

                match comm.send(format!("{}\n", jog_cmd).as_bytes()) {
                    Ok(_) => {
                        // Add to sent_lines for logging
                        if let Ok(mut gstate) = gcode_send_state_zp.lock() {
                            gstate.sent_lines.push_back(jog_cmd);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to send Jog Z+ command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Jog Z+ failed: {}", e),
                        );
                    }
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });

    // Set up machine-jog-z-negative callback
    let window_weak = main_window.as_weak();
    let communicator_clone = communicator.clone();
    let console_manager_clone = console_manager.clone();
    let gcode_send_state_zn = gcode_send_state.clone();
    main_window.on_machine_jog_z_negative(move |step_size: f32| {
        if let Some(window) = window_weak.upgrade() {
            let mut comm = communicator_clone.lock().unwrap();
            if !comm.is_connected() {
                warn!("Jog Z- failed: Device not connected");
                console_manager_clone
                    .add_message(DeviceMessageType::Error, "✗ Device not connected.");
            } else {
                // Send jog command in relative mode (G91) for incremental movement
                let jog_cmd = format!("$J=G91 Z-{} F2000", step_size);
                console_manager_clone.add_message(
                    DeviceMessageType::Output,
                    format!("Jogging Z- ({} mm)...", step_size),
                );

                match comm.send(format!("{}\n", jog_cmd).as_bytes()) {
                    Ok(_) => {
                        // Add to sent_lines for logging
                        if let Ok(mut gstate) = gcode_send_state_zn.lock() {
                            gstate.sent_lines.push_back(jog_cmd);
                        }
                    }
                    Err(e) => {
                        warn!("Failed to send Jog Z- command: {}", e);
                        console_manager_clone.add_message(
                            DeviceMessageType::Error,
                            format!("✗ Jog Z- failed: {}", e),
                        );
                    }
                }
            }

            let console_output = console_manager_clone.get_output();
            window.set_console_output(slint::SharedString::from(console_output));
        }
    });
}
