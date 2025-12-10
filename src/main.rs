use crate::logger::Logger;
use clap::{Parser, Subcommand};

pub mod client;
pub mod config;
pub mod file;
pub mod loading;
pub mod logger;
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
    let logger = Logger::new();

    match &cli.command {
        Commands::Config => match ui::select_mistral_model(&logger).await {
            Ok(model) => println!("Model switched to {}", model),
            Err(e) => {
                logger
                    .log_error(format!("Error switching model: {}", e))
                    .await
            }
        },
        Commands::Run { prompt } => {
            let escaped_prompt = prompt.replace("\"", "");
            let model_name = match config::get_model_name().await {
                Ok(model_name) => model_name,
                Err(e) => {
                    _ = ui::restore_terminal();
                    panic!("Model name error: {}", e);
                }
            };

            // Initialize the terminal for the UI
            let mut terminal = match ui::setup_terminal() {
                Ok(terminal) => terminal,
                Err(e) => {
                    _ = ui::restore_terminal();
                    panic!("Failed to setup terminal: {}", e);
                }
            };

            // Create a channel for real-time updates
            let (sender, receiver) = tokio::sync::mpsc::channel(100);

            // Spawn a task to handle real-time updates
            let prompt_clone = escaped_prompt.clone();
            let logger_clone = logger.clone();
            let display_task = tokio::spawn(async move {
                if let Err(_) =
                    client::call_mistral_completions(prompt_clone, sender, &logger_clone).await
                {
                    // TODO: Handle error
                }
            });

            // Render the UI
            if let Err(e) =
                ui::render_ui(&mut terminal, &logger, model_name, escaped_prompt, receiver).await
            {
                logger.log_error(format!("{}", e)).await;
            }

            // Wait for the display task to complete
            display_task.await.unwrap();

            // // Restore the terminal
            // if let Err(e) = ui::restore_terminal() {
            //     eprintln!("Failed to restore terminal: {}", e);
            //     std::process::exit(1);
            // }
        }
    }
}
