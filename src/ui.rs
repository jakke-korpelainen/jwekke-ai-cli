use crate::{client::list_mistral_models, config::save_model_name};
use inquire::{InquireError, Select};

/// Displays a list of available Mistral AI models and prompts the user to select one.
///
/// # Returns
/// A `Result` containing the selected model's name or an error.
pub async fn select_mistral_model() -> Result<String, Box<dyn std::error::Error>> {
    let models = list_mistral_models().await.map_err(|e| {
        eprintln!("Failed to load models: {}", e);
        e
    })?;

    let options = models
        .iter()
        .map(|model| model.id.as_str())
        .collect::<Vec<&str>>();
    let ans: Result<&str, InquireError> = Select::new("Select Mistral Model", options)
        .with_page_size(15)
        .prompt();
    let selection = match ans {
        Ok(choice) => {
            println!("Selected model: {}", choice);
            choice.to_string()
        }
        Err(_) => {
            println!("There was an error, please try again");
            return Err("Selection failed".into());
        }
    };

    save_model_name(selection.clone()).await.map_err(|e| {
        eprintln!("Failed to save model to config: {}", e);
        e
    })?;

    Ok(selection)
}
