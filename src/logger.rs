use crate::file;
use bytes::Bytes;
use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};
use tokio::{fs::File, io::AsyncWriteExt, sync::Mutex};

#[derive(Debug, Clone)]
pub struct Logger {
    errors: Arc<Mutex<Vec<String>>>,
    stream_log_file: Arc<Mutex<File>>,
    error_log_file: Arc<Mutex<File>>,
}

// entries: epoch|content
fn compare_error_log_line(a: &String, b: &String) -> std::cmp::Ordering {
    let epoch_a = a
        .split('|')
        .next()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);
    let epoch_b = b
        .split('|')
        .next()
        .unwrap_or("0")
        .parse::<u64>()
        .unwrap_or(0);
    epoch_a.cmp(&epoch_b)
}

impl Logger {
    pub async fn new() -> Self {
        Logger {
            stream_log_file: Arc::new(Mutex::new(file::create_stream_log_file().await.1)),
            error_log_file: Arc::new(Mutex::new(file::create_error_log_file().await.1)),
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn write_stream_log(&self, bytes: Bytes) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut buffer = Vec::new();
        buffer.extend_from_slice(format!("{now}|").as_bytes());
        buffer.extend_from_slice(&bytes);
        buffer.push(b'\n');

        self.stream_log_file
            .lock()
            .await
            .write_all(&buffer)
            .await
            .unwrap_or_else(|e| panic!("{e}"));
    }

    pub async fn get_sorted_errors(&self) -> Vec<String> {
        // Sort the errors based on the epoch before writing
        let mut errors = self.get_errors().await;
        errors.sort_by(compare_error_log_line);
        errors
    }

    pub async fn log_error(&self, error: String) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_millis();

        let mut errors = self.errors.lock().await;
        errors.push(format!("{}|{}", now, error));
        drop(errors);

        let errors = &self.get_sorted_errors().await;
        self.error_log_file.lock().await.flush().await.unwrap();
        self.error_log_file
            .lock()
            .await
            .write_all(errors.join("\n").as_bytes())
            .await
            .unwrap();
    }

    pub async fn get_errors(&self) -> Vec<String> {
        let errors = self.errors.lock().await;
        errors.clone()
    }

    pub async fn clear_errors(&self) {
        let mut errors = self.errors.lock().await;
        errors.clear();
    }
}

impl Default for Logger {
    fn default() -> Self {
        tokio::runtime::Runtime::new()
            .unwrap()
            .block_on(Self::new())
    }
}
