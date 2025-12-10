use crate::loading::spawn_spinner;
use crate::models::{MistralModelCard, MistralModelResponse};
use crate::{config, stream};
use reqwest::Client;
use std::env;
use std::io::Write;
use tokio::sync::{mpsc, watch};

const API_URL: &'static str = "https://api.mistral.ai/v1/chat/completions";
const DEFAULT_API_MODEL: &'static str = "mistral-tiny";

pub async fn list_mistral_models() -> Result<Vec<MistralModelCard>, Box<dyn std::error::Error>> {
    let mistral_api_key = env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");
    let client = Client::new();

    let response = match client
        .get("https://api.mistral.ai/v1/models")
        .header("Authorization", format!("Bearer {}", mistral_api_key))
        .send()
        .await
    {
        Ok(response) => {
            if !response.status().is_success() {
                eprintln!("Client Error: {}", response.status());
                panic!("{}", response.status());
            }

            response
                .json::<MistralModelResponse>()
                .await
                .expect("Failed to read json")
                .data
        }
        Err(e) => {
            eprint!("Request failed for Mistral models: {}", e);
            panic!("{}", e);
        }
    };

    // filter only models with chat completion

    Ok(response
        .into_iter()
        .filter(|model| model.capabilities.completion_chat == true)
        .collect())
}

pub async fn call_mistral_completions(prompt: String) -> Result<(), Box<dyn std::error::Error>> {
    let mistral_model = config::get_model_name()
        .await
        .unwrap_or_else(|_| DEFAULT_API_MODEL.to_string());
    let mistral_api_key = env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");
    let client = Client::new();

    let request_body = serde_json::json!({
        "model": mistral_model,
        "messages": [{"role": "user", "content": prompt}],
        "stream": true
    });

    // Create a channel to control the spinner
    let (tx, rx) = watch::channel(true);

    // Spawn the spinner as a Tokio task
    let spinner_task = spawn_spinner(rx, "Connecting to Mistral API".to_string());

    let response = match client
        .post(API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", mistral_api_key))
        .json(&request_body)
        .send()
        .await
    {
        Ok(response) => {
            let status = response.status();
            println!("Response Status: {}", status);
            response
        }
        Err(e) => {
            eprintln!("Error sending request: {}", e);
            return Err(e.into());
        }
    };

    // Stop the spinner
    tx.send(false)?;

    // Ensure the spinner task completes
    spinner_task.await?;

    // Create a channel for real-time updates
    let (sender, mut receiver) = mpsc::channel(100);

    // Spawn a task to handle real-time updates
    let display_task = tokio::spawn(async move {
        while let Some(chunk) = receiver.recv().await {
            print!("{}", chunk);
            std::io::stdout().flush().unwrap();
        }
    });

    stream::parse_mistral_stream(Box::pin(response.bytes_stream()), sender)
        .await
        .expect("Result stream chunking failed");

    // Wait for the display task to complete
    display_task.await.unwrap();

    Ok(())
}
