//! Feed rate and spindle override handlers

use super::*;

impl MachineControlView {
    // Feed Rate Override Controls Setup
    pub(crate) fn setup_override_handlers(view: &Self) {
        // Feed Rate Override Controls
        // GRBL Realtime Commands for Feed Override:
        // 0x90 = Reset to 100%
        // 0x91 = +10%
        // 0x92 = -10%
        // 0x93 = +1%
        // 0x94 = -1%
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_inc10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed +10%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x91]); // Feed +10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_inc1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed +1%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x93]); // Feed +1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_dec1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed -1%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x94]); // Feed -1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_dec10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed -10%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x92]); // Feed -10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.feed_reset.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Feed Reset (100%)\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x90]); // Feed override reset to 100%
                }
            });
        }

        // Spindle Override Controls
        // GRBL Realtime Commands for Spindle Override:
        // 0x99 = 100% (Reset)
        // 0x9A = +10%
        // 0x9B = -10%
        // 0x9C = +1%
        // 0x9D = -1%
        // 0x9E = Spindle Stop (Toggle)
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_inc10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle +10%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x9A]); // Spindle +10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_inc1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle +1%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x9C]); // Spindle +1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_dec1.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle -1%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x9D]); // Spindle -1%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_dec10.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle -10%\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x9B]); // Spindle -10%
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_stop.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle Stop\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x9E]); // Spindle stop
                }
            });
        }
        {
            let communicator = view.communicator.clone();
            let console = view.device_console.clone();
            view.spindle_reset.connect_clicked(move |_| {
                if let Some(c) = console.as_ref() {
                    c.append_log("> Spindle Reset (100%)\n");
                }
                if let Ok(mut comm) = communicator.lock() {
                    let _ = comm.send(&[0x99]); // Spindle override reset to 100%
                }
            });
        }
    }
}
