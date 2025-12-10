use std::path::PathBuf;
use tokio::fs::{File, OpenOptions};

const CLI_DIR: &str = ".config/jwekke-ai-cli";
const CACHE_FILE_NAME: &str = "cache.jwek";
const CONFIG_FILE_NAME: &str = "config.jwek";

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

pub fn get_cache_file_path() -> PathBuf {
    let dir_path = get_cli_config_dir();
    dir_path.join(CACHE_FILE_NAME)
}

pub fn get_config_file_path() -> PathBuf {
    let dir_path = get_cli_config_dir();
    dir_path.join(CONFIG_FILE_NAME)
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

pub async fn open_cache_file() -> (PathBuf, File) {
    let cache_file_path = get_cache_file_path();

    match open_file(cache_file_path).await {
        Ok((file_path, file)) => (file_path, file),
        Err(e) => {
            panic!("Failed to open cache file: {}", e);
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

pub async fn create_cache_file() -> (PathBuf, File) {
    let cache_file_path = get_cache_file_path();
    match create_file(cache_file_path, true).await {
        Ok(file) => file,
        Err(e) => panic!("Failed to create cache file: {}", e),
    }
}

pub async fn create_config_file() -> (PathBuf, File) {
    let config_file_path = get_config_file_path();
    match create_file(config_file_path, true).await {
        Ok(file) => file,
        Err(e) => panic!("Failed to create config file: {}", e),
    }
}
