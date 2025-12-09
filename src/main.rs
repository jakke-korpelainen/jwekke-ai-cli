use crate::client::call_mistral_completions;
use std::env;

pub mod client;
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

    if let Err(e) = call_mistral_completions(prompt.to_string()).await {
        eprintln!("Error: {}", e);
        std::process::exit(1);
    }
}
