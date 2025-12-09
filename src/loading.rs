use std::time::Duration;

use tokio::{
    io::{self, AsyncWriteExt},
    sync::watch::Receiver,
    time,
};

const SPINNER_FRAMES: [&str; 10] = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];

pub fn spawn_spinner(rx: Receiver<bool>, label: String) -> tokio::task::JoinHandle<()> {
    let spinner_task = tokio::spawn(async move {
        let mut i = 0;
        let mut stdout = io::stdout();
        while *rx.borrow() {
            let _ = stdout
                .write_all(format!("\r{} {}...", SPINNER_FRAMES[i], label).as_bytes())
                .await;
            let _ = stdout.flush().await;
            i = (i + 1) % SPINNER_FRAMES.len();
            time::sleep(Duration::from_millis(100)).await;
        }

        let _ = stdout.write_all(b"\r").await;
        let _ = stdout.flush().await;
    });
    spinner_task
}
