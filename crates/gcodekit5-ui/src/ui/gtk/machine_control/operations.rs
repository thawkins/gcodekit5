//! Machine control operations
#![allow(deprecated)]

use super::*;

impl MachineControlView {
    pub fn refresh_ports(&self) {
        self.port_combo.remove_all();

        match gcodekit5_communication::list_ports() {
            Ok(ports) if !ports.is_empty() => {
                for port in ports {
                    self.port_combo
                        .append(Some(&port.port_name), &port.port_name);
                }
                // Select the first port
                self.port_combo.set_active(Some(0));
            }
            _ => {
                self.port_combo
                    .append(Some("none"), &t!("No ports available"));
                self.port_combo.set_active_id(Some("none"));
            }
        }
    }

    pub fn get_step_size(&self) -> f64 {
        *self.jog_step_mm.lock() as f64
    }

    pub fn start_job(&self, content: &str) {
        if *self.is_streaming.lock() {
            return;
        }

        let lines: Vec<String> = content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with(';') && !s.starts_with('('))
            .collect();

        if lines.is_empty() {
            if let Some(c) = self.device_console.as_ref() {
                c.append_log(&format!("{}\n", t!("No valid G-Code lines found.")));
            }
            return;
        }

        // Scan first 100 lines for F and S commands to set initial commanded values
        let mut found_f = false;
        let mut found_s = false;
        for line in lines.iter().take(100) {
            if found_f && found_s {
                break;
            }

            let upper = line.to_ascii_uppercase();

            if !found_f {
                if let Some(idx) = upper.find('F') {
                    let rest = &upper[idx + 1..];
                    let val_str: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    if let Ok(val) = val_str.parse::<f32>() {
                        device_status::update_commanded_feed_rate(val);
                        found_f = true;
                    }
                }
            }

            if !found_s {
                if let Some(idx) = upper.find('S') {
                    let rest = &upper[idx + 1..];
                    let val_str: String = rest
                        .chars()
                        .take_while(|c| c.is_ascii_digit() || *c == '.')
                        .collect();
                    if let Ok(val) = val_str.parse::<f32>() {
                        device_status::update_commanded_spindle_speed(val);
                        found_s = true;
                    }
                }
            }
        }

        {
            let mut queue = self.send_queue.lock();
            queue.clear();
            for line in lines.iter() {
                queue.push_back(line.clone());
            }
            *self.total_lines.lock() = queue.len();
        }

        *self.is_streaming.lock() = true;
        *self.is_paused.lock() = false;
        *self.waiting_for_ack.lock() = false;
        *self.job_start_time.lock() = Some(std::time::Instant::now());

        // Kickstart
        {
            let mut comm = self.communicator.lock();
            let mut queue = self.send_queue.lock();
            if let Some(cmd) = queue.pop_front() {
                if let Some(c) = self.device_console.as_ref() {
                    c.append_log(&format!("> {}\n", cmd));
                }
                let _ = comm.send_command(&cmd);
                *self.waiting_for_ack.lock() = true;
            }
        }
    }

    pub fn emergency_stop(&self) {
        {
            let mut comm = self.communicator.lock();
            let _ = comm.send(&[0x18]);
        }

        *self.is_streaming.lock() = false;
        *self.is_paused.lock() = false;
        *self.waiting_for_ack.lock() = false;
        *self.job_start_time.lock() = None;
        self.send_queue.lock().clear();

        // Reset progress
        if let Some(sb) = self.status_bar.as_ref() {
            sb.set_progress(0.0, "", "");
        }

        // Match StatusBar eStop behavior
        if let Some(c) = self.device_console.as_ref() {
            c.append_log(&format!("{}\n", t!("Emergency stop (Ctrl-X)")));
        }
    }
}
