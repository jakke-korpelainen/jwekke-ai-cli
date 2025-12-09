use std::path::PathBuf;

use tokio::{
    fs::{File, OpenOptions, remove_file},
    io::AsyncWriteExt,
};

const CACHE_FILE_NAME: &str = "cache.txt";

// ~/.local/jwekke-ai-cli/cache
pub fn get_file_cache_path() -> PathBuf {
    let home_dir = dirs::home_dir().expect("Failed to get home directory");

    let dir_path = home_dir.join(".local/jwekke-ai-cli");
    std::fs::create_dir_all(&dir_path).expect("Failed to create directory");

    dir_path.join(CACHE_FILE_NAME)
}

pub async fn open_file_cache() -> (PathBuf, File) {
    let file_path = get_file_cache_path();

    // Open the file in append mode
    let file = OpenOptions::new()
        .append(true)
        .create(true)
        .open(&file_path)
        .await
        .expect("Can't open file");

    (file_path, file)
}

pub async fn create_file_cache() -> (PathBuf, File) {
    let file_path = get_file_cache_path();
    if file_path.exists() {
        remove_file(&file_path)
            .await
            .expect("Failed to remove existing cache file");
    }

    // Create the file if it doesn't exist
    let mut file = OpenOptions::new()
        .create(true)
        .write(true)
        .open(&file_path)
        .await
        .expect("Can't write to file");

    file.write(b"").await.expect("Failed to write to file");

    (file_path, file)
}
