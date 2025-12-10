use std::sync::Arc;
use tokio::sync::Mutex;

#[derive(Debug, Clone)]
pub struct Logger {
    errors: Arc<Mutex<Vec<String>>>,
}

impl Logger {
    pub fn new() -> Self {
        Logger {
            errors: Arc::new(Mutex::new(Vec::new())),
        }
    }

    pub async fn log_error(&self, error: String) {
        let mut errors = self.errors.lock().await;
        errors.push(error);
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
        Self::new()
    }
}
