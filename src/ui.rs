// ui.rs

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{client::list_mistral_models, config::save_model_name};

pub const COMMANDS_CONFIG: &'static str = "config";

/// Reads user input from stdin asynchronously.
///
/// # Returns
/// A `String` containing the user's input.
async fn read_user_input() -> String {
    let stdin = io::stdin();
    let mut buf_reader = BufReader::new(stdin);

    let mut line = String::new();
    if let Ok(_) = buf_reader.read_line(&mut line).await {
        line
    } else {
        "".to_string()
    }
}

/// Displays a list of available Mistral AI models and prompts the user to select one.
///
/// # Returns
/// A `Result` containing the selected model's name or an error.
pub async fn prompt_user_for_selection() -> Result<String, Box<dyn std::error::Error>> {
    let models = list_mistral_models().await.map_err(|e| {
        eprintln!("Failed to load models: {}", e);
        e
    })?;

    display_models(&models).await?;

    let selection = prompt_for_selection(models.len()).await?;

    let model_name = models[selection - 1].id.clone();
    save_model_name(model_name.clone()).await.map_err(|e| {
        eprintln!("Failed to save model to config: {}", e);
        e
    })?;

    Ok(model_name)
}

/// Displays a list of Mistral AI models to the user.
///
/// # Arguments
/// * `models` - A slice of `MistralModelCard` objects representing the available models.
///
/// # Returns
/// A `Result` indicating success or failure.
async fn display_models(models: &[crate::models::MistralModelCard]) -> Result<(), std::io::Error> {
    for (i, model) in models.iter().enumerate() {
        println!(
            "({}) {}\n{}\n",
            i + 1,
            model.id,
            model.description.as_deref().unwrap_or("N/A")
        );
    }
    tokio::io::stdout().flush().await?;
    Ok(())
}

/// Prompts the user to select a model by entering a number.
///
/// # Arguments
/// * `max_selection` - The maximum valid selection number.
///
/// # Returns
/// A `Result` containing the user's selection or an error.
async fn prompt_for_selection(max_selection: usize) -> Result<usize, Box<dyn std::error::Error>> {
    println!("-----");
    println!("Select a model (1-{}):", max_selection);

    loop {
        let input = read_user_input().await;
        match input.trim().parse::<usize>() {
            Ok(num) if num >= 1 && num <= max_selection => {
                return Ok(num);
            }
            _ => {
                println!(
                    "Invalid selection. Please enter a number between 1 and {}.",
                    max_selection
                );
            }
        }
    }
}
