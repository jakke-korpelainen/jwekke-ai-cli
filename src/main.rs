use clap::{Parser, Subcommand};

pub mod client;
pub mod config;
pub mod file;
pub mod loading;
pub mod models;
pub mod stream;
pub mod ui;

/// A CLI tool for interacting with the Mistral AI API
#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a prompt through the Mistral AI API
    Run {
        /// The prompt to send to the Mistral AI API
        prompt: String,
    },
    /// Configure the Mistral AI model
    Config,
}

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Config => match ui::select_mistral_model().await {
            Ok(model) => println!("Model switched to {}", model),
            Err(e) => eprintln!("Model switch error: {}", e),
        },
        Commands::Run { prompt } => {
            let escaped_prompt = prompt.replace("\"", "");

            let model_name = match config::get_model_name().await {
                Ok(model_name) => model_name,
                Err(e) => {
                    eprintln!("Model name error: {}", e);
                    std::process::exit(1);
                }
            };
            println!("Using model: {}", model_name);

            if let Err(e) = client::call_mistral_completions(escaped_prompt).await {
                eprintln!("Mistral completions error: {}", e);
                std::process::exit(1);
            }
        }
    }
}
