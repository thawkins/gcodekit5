//! direct_sender.rs // Improved file for raster engraving

use crate::t;
use gcodekit5_communication::{Communicator, SerialCommunicator};
use gcodekit5_core::ThreadSafe;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::Duration;

pub struct DirectSender {
    communicator: ThreadSafe<SerialCommunicator>,
    is_streaming: Arc<AtomicBool>,
    is_paused: Arc<AtomicBool>,
    should_stop: Arc<AtomicBool>,
    progress_tx: mpsc::Sender<String>,
}

impl DirectSender {
    pub fn new(
        communicator: ThreadSafe<SerialCommunicator>,
        is_streaming: ThreadSafe<bool>,
        is_paused: ThreadSafe<bool>,
        _waiting_for_ack: ThreadSafe<bool>,
    ) -> (Self, mpsc::Receiver<String>) {
        let streaming_flag = Arc::new(AtomicBool::new(*is_streaming.lock()));
        let paused_flag = Arc::new(AtomicBool::new(*is_paused.lock()));
        let stop_flag = Arc::new(AtomicBool::new(false));

        *is_streaming.lock() = streaming_flag.load(Ordering::SeqCst);
        *is_paused.lock() = paused_flag.load(Ordering::SeqCst);

        let (tx, rx) = mpsc::channel();
//        println!("DEBUG: DirectSender");
        (
            Self {
                communicator,
                is_streaming: streaming_flag,
                is_paused: paused_flag,
                should_stop: stop_flag,
                progress_tx: tx,
            },
            rx,
        )
    }

    fn send_progress(&self, message: String) {
        let _ = self.progress_tx.send(message);
    }

    pub fn send_gcode(&self, content: &str) {
        if self.is_streaming.load(Ordering::SeqCst) {
            return;
        }

        let lines: Vec<String> = content
            .lines()
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty() && !s.starts_with(';') && !s.starts_with('('))
            .collect();

        if lines.is_empty() {
            return;
        }

        let total_lines = lines.len();
        self.send_progress(format!(
            "{} {} {}",
            t!("Starting laser engraving:"),
            total_lines,
            t!("lines")
        ));

        self.is_streaming.store(true, Ordering::SeqCst);
        self.is_paused.store(false, Ordering::SeqCst);
        self.should_stop.store(false, Ordering::SeqCst);

        let comm = self.communicator.clone();
        let streaming_flag = self.is_streaming.clone();
        let paused_flag = self.is_paused.clone();
        let stop_flag = self.should_stop.clone();
        let progress_tx = self.progress_tx.clone();

        thread::spawn(move || {
            let mut lines_in_flight: usize = 0;
            let max_window: usize = 40;
            let mut i = 0;
            let total = lines.len();
            let mut last_percent: u32 = 0;

            while i < total && !stop_flag.load(Ordering::SeqCst) {
                // SEND until the window is full

                while lines_in_flight < max_window && i < total {
                    if paused_flag.load(Ordering::SeqCst) {
                        break;
                    }
                    let cmd = format!("{}\n", lines[i]);

                    let _ = progress_tx.send(format!("> {}", lines[i]));

                    let mut c = comm.lock();
                    if c.send(cmd.as_bytes()).is_ok() {
                        lines_in_flight += 1;
                        i += 1;
                    } else {
                        break;
                    }
                }

                // If the window is full, force waiting for a real 'ok'
                let mut attempts = 0;
                while lines_in_flight >= max_window && !stop_flag.load(Ordering::SeqCst) {
                    {
                        let mut c = comm.lock();
                        if let Ok(data) = c.receive() {
                            let resp = String::from_utf8_lossy(&data);
                            let ok_count = resp.matches("ok").count();
                            lines_in_flight = lines_in_flight.saturating_sub(ok_count);
                        }
                    }

                    attempts += 1;
                    if attempts > 100 {
                        // If after many attempts there is no 'ok', we give the CPU a break.
                        thread::sleep(Duration::from_millis(1));
                        attempts = 0;
                    }
                }

                // Read whatever has arrived even if the window is not full
                if let Ok(data) = {
                    let mut c = comm.lock();
                    // We use a non-blocking or fast read
                    c.receive()
                } {
                    if !data.is_empty() {
                        // We only process data if there is actually data.
                        let resp = String::from_utf8_lossy(&data);

                        if resp.contains("Grbl") || resp.contains("ALARM") {
                            stop_flag.store(true, Ordering::SeqCst);
                            break;
                        }

                        let ok_count = resp.matches("ok").count();
                        lines_in_flight = lines_in_flight.saturating_sub(ok_count);
                    }
                }
                thread::yield_now();

                let percent = ((i as f64 / total as f64) * 100.0)
                    .round()
                    .clamp(0.0, 100.0) as u32;
                if percent != last_percent {
                    last_percent = percent;
                    let _ = progress_tx.send(format!("* {}%", percent));
                }
            }
            streaming_flag.store(false, Ordering::SeqCst);
        });
    }

    pub fn stop(&self) {
        let _ = self.progress_tx.send(t!("Stopping...").to_string());
        self.should_stop.store(true, Ordering::SeqCst);
    }

    pub fn pause(&self) {
        let _ = self.progress_tx.send(t!("Paused").to_string());
        self.is_paused.store(true, Ordering::SeqCst);
    }

    pub fn resume(&self) {
        let _ = self.progress_tx.send(t!("Resuming").to_string());
        self.is_paused.store(false, Ordering::SeqCst);
    }

    pub fn unlock(&self) {
        let _ = self.progress_tx.send(t!("Unlocking...").to_string());

        let comm = self.communicator.clone();
        let _ = thread::spawn(move || {
            let mut comm = comm.lock();
            let _ = comm.send_command("$X");
        });
    }

    /// Calcula el tiempo estimado de ejecución en segundos analizando el G-code
    pub fn estimate_execution_time(gcode: &str) -> f64 {
        let mut total_time = 0.0;
        let mut current_feed = 0.0; // mm/min
        let mut last_x = 0.0;
        let mut last_y = 0.0;
        let mut last_z = 0.0;
        let mut is_absolute = true; // G90 por defecto

        for line in gcode.lines() {
            let line = line.trim();
            if line.is_empty() || line.starts_with(';') {
                continue;
            }

            // Detectar modo de posicionamiento
            if line.contains("G90") {
                is_absolute = true;
            } else if line.contains("G91") {
                is_absolute = false;
            }

            // Detectar velocidad de avance
            if let Some(f_pos) = line.find('F') {
                let f_end = line[f_pos+1..].find(|c: char| !c.is_ascii_digit() && c != '.')
                    .map_or(line.len(), |idx| f_pos + 1 + idx);
                if let Ok(feed) = line[f_pos+1..f_end].parse::<f64>() {
                    current_feed = feed;
                }
            }

            // Detectar movimiento lineal G0, G1
            let is_move = line.contains("G0") || line.contains("G1") || line.contains("G00") || line.contains("G01");

            if is_move {
                let mut x = last_x;
                let mut y = last_y;
                let mut z = last_z;

                // Extraer coordenadas
                if let Some(x_pos) = line.find('X') {
                    let x_end = line[x_pos+1..].find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                        .map_or(line.len(), |idx| x_pos + 1 + idx);
                    if let Ok(val) = line[x_pos+1..x_end].parse::<f64>() {
                        x = if is_absolute { val } else { last_x + val };
                    }
                }

                if let Some(y_pos) = line.find('Y') {
                    let y_end = line[y_pos+1..].find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                        .map_or(line.len(), |idx| y_pos + 1 + idx);
                    if let Ok(val) = line[y_pos+1..y_end].parse::<f64>() {
                        y = if is_absolute { val } else { last_y + val };
                    }
                }

                if let Some(z_pos) = line.find('Z') {
                    let z_end = line[z_pos+1..].find(|c: char| !c.is_ascii_digit() && c != '.' && c != '-')
                        .map_or(line.len(), |idx| z_pos + 1 + idx);
                    if let Ok(val) = line[z_pos+1..z_end].parse::<f64>() {
                        z = if is_absolute { val } else { last_z + val };
                    }
                }

                // Calcular distancia (convertir a f64 explícitamente)
                let dx = x - last_x;
                let dy = y - last_y;
                let dz = z - last_z;
                let distance = (dx * dx + dy * dy + dz * dz).sqrt();

                if distance > 0.0 && current_feed > 0.0 {
                    // tiempo = distancia / velocidad (convertir mm/min a mm/seg)
                    total_time += distance / (current_feed / 60.0);
                }

                last_x = x;
                last_y = y;
                last_z = z;
            }
        }

        total_time
    }
}

impl Clone for DirectSender {
    fn clone(&self) -> Self {
        Self {
            communicator: self.communicator.clone(),
            is_streaming: self.is_streaming.clone(),
            is_paused: self.is_paused.clone(),
            should_stop: self.should_stop.clone(),
            progress_tx: self.progress_tx.clone(),
        }
    }
}
