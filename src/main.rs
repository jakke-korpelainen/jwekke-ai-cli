use crate::{client::call_mistral_completions, file::create_file_cache};
use std::env;
use std::process::Stdio;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, stdout};
use tokio::process::Command as AsyncCommand;

pub mod client;
pub mod file;
pub mod loading;
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
    let (file_path, _) = create_file_cache().await;

    if let Err(e) = call_mistral_completions(prompt.to_string()).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }

    // Read the file content and pass it to `glow` via stdin
    let mut file = File::open(&file_path).await.expect("Failed to open cache");
    let mut input_buffer = Vec::new();
    file.read_to_end(&mut input_buffer)
        .await
        .expect("Failed to read file");

    let mut glow_process = AsyncCommand::new("glow")
        .arg("-s")
        .arg("tokyo-night")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .spawn()
        .expect("Failed to spawn glow command");

    if let Some(stdin) = glow_process.stdin.as_mut() {
        stdin
            .write_all(format!("Prompt: {}\n___\n", prompt).as_bytes())
            .await
            .expect("Failed to write to stdin");
        stdin
            .write_all(&input_buffer)
            .await
            .expect("Failed to write to stdin");
    }

    let output = glow_process
        .wait_with_output()
        .await
        .expect("Failed to wait for glow process");

    // Clear the console and display the output
    print!("\x1B[2J\x1B[1;1H"); // ANSI escape codes to clear screen

    stdout()
        .write_all(&output.stdout)
        .await
        .expect("Failed to write output");
}
