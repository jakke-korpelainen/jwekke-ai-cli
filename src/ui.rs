// ui.rs

use tokio::io::{self, AsyncBufReadExt, AsyncWriteExt, BufReader};

use crate::{client::list_mistral_models, config::save_model_name};

pub const COMMANDS_CONFIG: &'static str = "config";

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

pub async fn prompt_user_for_selection() -> Result<String, std::io::Error> {
    let models = list_mistral_models().await.expect("Failed to load models");
    for (i, model) in models.iter().enumerate() {
        println!(
            "({}) {}\n{}\n",
            i + 1,
            model.id,
            model.description.as_deref().unwrap_or("N/A")
        );
    }
    tokio::io::stdout()
        .flush()
        .await
        .expect("Failed to flush stdout");

    println!("-----");
    println!("Select a model (1-{}):", models.len());
    let input = read_user_input().await;

    match input.trim().parse::<usize>() {
        Ok(num) if num >= 1 && num <= models.len() => {
            let model_name = models[num - 1].id.clone();
            match save_model_name(model_name.clone()).await {
                Ok(_) => {}
                Err(e) => panic!("Failed to save model to config: {}", e),
            }
            Ok(model_name)
        }
        _ => {
            println!(
                "Invalid selection. Please enter a number between 1 and {}.",
                models.len()
            );
            Box::pin(prompt_user_for_selection()).await
        }
    }
}
