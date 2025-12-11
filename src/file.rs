use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};

const CLI_DIR: &str = ".config/jwekke-ai-cli";
const CONFIG_FILE_NAME: &str = "config.jwek";
const STREAM_LOG_FILE_PATH: &str = "stream.log";
const ERROR_LOG_FILE_PATH: &str = "error.log";

pub fn get_cli_config_dir() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");

    let dir_path = home_dir.join(CLI_DIR);
    match std::fs::create_dir_all(&dir_path) {
        Ok(_) => dir_path,
        Err(e) => {
            panic!("Failed to create config directory: {}", e);
        }
    }
}

pub async fn create_file(
    file_path: PathBuf,
    truncate: bool,
) -> Result<(PathBuf, File), std::io::Error> {
    match OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(truncate)
        .open(&file_path)
        .await
    {
        Ok(file) => Ok((file_path, file)),
        Err(e) => {
            panic!("Failed to create file: {:?}, error: {}", file_path, e);
        }
    }
}

pub async fn open_file(file_path: PathBuf) -> Result<(PathBuf, File), std::io::Error> {
    match OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_path)
        .await
    {
        Ok(file) => Ok((file_path, file)),
        Err(e) => {
            panic!("Failed to open file: {:?}, error: {}", file_path, e);
        }
    }
}

pub fn get_config_file_path() -> PathBuf {
    let dir_path = get_cli_config_dir();
    dir_path.join(CONFIG_FILE_NAME)
}

pub fn get_error_log_file_path() -> PathBuf {
    let dir_path = get_cli_config_dir();
    dir_path.join(ERROR_LOG_FILE_PATH)
}

pub fn get_stream_log_file_path() -> PathBuf {
    let dir_path = get_cli_config_dir();
    dir_path.join(STREAM_LOG_FILE_PATH)
}

pub async fn open_config_file() -> (PathBuf, File) {
    let config_file_path = get_config_file_path();

    match open_file(config_file_path).await {
        Ok((file_path, file)) => (file_path, file),
        Err(e) => {
            panic!("Failed to open config file: {}", e);
        }
    }
}

pub async fn create_stream_log_file() -> (PathBuf, File) {
    let stream_log_file_path = get_stream_log_file_path();
    match create_file(stream_log_file_path, true).await {
        Ok(file) => file,
        Err(e) => panic!("Failed to create log file: {}", e),
    }
}

pub async fn create_error_log_file() -> (PathBuf, File) {
    let error_log_file_path = get_error_log_file_path();
    match create_file(error_log_file_path, true).await {
        Ok(file) => file,
        Err(e) => panic!("Failed to create log file: {}", e),
    }
}

pub async fn create_config_file(truncate: Option<bool>) -> (PathBuf, File) {
    let config_file_path = get_config_file_path();
    match create_file(config_file_path, truncate.unwrap_or_else(|| true)).await {
        Ok(file) => file,
        Err(e) => panic!("Failed to create config file: {}", e),
    }
}
