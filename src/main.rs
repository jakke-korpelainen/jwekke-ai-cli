use crate::{client::call_mistral_completions, file::create_file_cache};
use std::env;
use tokio::process::Command;

pub mod client;
pub mod file;
pub mod models;
pub mod stream;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <prompt>", args[0]);
        std::process::exit(1);
    }
    let prompt = &args[1];

    let (file_path, _) = create_file_cache();

    if let Err(e) = call_mistral_completions(prompt.to_string()).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    let mut child = Command::new("glow")
        .arg("-s")
        .arg("tokyo-night")
        .arg(
            file_path
                .to_str()
                .expect("Failed to convert path to string"),
        )
        .spawn()
        .expect("Failed to spawn glow command");

    let _ = child.wait().await;
}
