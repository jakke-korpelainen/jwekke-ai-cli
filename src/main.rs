use std::env;
use std::process::Stdio;
use tokio::fs::File;
use tokio::io::{AsyncReadExt, AsyncWriteExt, stdout};
use tokio::process::Command as AsyncCommand;

pub mod client;
pub mod config;
pub mod file;
pub mod loading;
pub mod models;
pub mod stream;
pub mod ui;

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 {
        eprintln!("Usage: {} <prompt>", args[0]);
        std::process::exit(1);
    }
    if &args[1] == ui::COMMANDS_CONFIG {
        match ui::prompt_user_for_selection().await {
            Ok(model) => println!("Model switched to {}", model),
            Err(e) => eprintln!("Model switch error: {}", e),
        }
        return;
    }

    let prompt = &args[1];

    let model_name = match config::get_model_name().await {
        Ok(model_name) => model_name,
        Err(e) => {
            eprintln!("Model name error: {}", e);
            std::process::exit(1);
        }
    };
    println!("Using model: {}", model_name);

    let (cache_file_path, _) = file::create_cache_file().await;

    if let Err(e) = client::call_mistral_completions(prompt.to_string()).await {
        eprintln!("Mistral completions error: {}", e);
        std::process::exit(1);
    }

    // Read the file content and pass it to `glow` via stdin
    let mut file = File::open(&cache_file_path)
        .await
        .expect("Failed to open cache");
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
        let model_name = match config::get_model_name().await {
            Ok(model) => model,
            Err(e) => panic!("Failed to get model name: {}", e),
        };
        stdin
            .write_all(format!("Model: {}, Prompt: {}\n___\n", model_name, prompt).as_bytes())
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
