use std::io::{IsTerminal, Write, stdout};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const FRAMES: [char; 4] = ['|', '/', '-', '\\'];
const INTERVAL: Duration = Duration::from_millis(50);

#[derive(Debug, Default)]
pub struct Spinner {
    requested_stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Spinner {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn start(&mut self) {
        // Keep the output clean when it is redirected to a file or a pipe.
        if !stdout().is_terminal() {
            return;
        }

        let requested_stop = Arc::clone(&self.requested_stop);
        self.handle = Some(thread::spawn(move || {
            let mut out = stdout().lock();
            for frame in FRAMES.iter().cycle() {
                if requested_stop.load(Ordering::Relaxed) {
                    break;
                }

                let status = format!("{frame} computing");
                let _ = out.write_all(status.as_bytes());
                let _ = out.flush();
                let _ = out.write_all("\u{8}".repeat(status.len()).as_bytes());
                thread::sleep(INTERVAL);
            }
        }));
    }

    pub fn stop(&mut self) {
        self.requested_stop.store(true, Ordering::Relaxed);

        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

impl Drop for Spinner {
    fn drop(&mut self) {
        self.stop();
    }
}
