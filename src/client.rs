use crate::loading::spawn_spinner;
use crate::{file::open_file_cache, stream::parse_mistral_stream};
use reqwest::Client;
use tokio::sync::watch::{self};

const API_URL: &'static str = "https://api.mistral.ai/v1/chat/completions";
const API_MODEL: &'static str = "mistral-tiny";

pub async fn call_mistral_completions(prompt: String) -> Result<(), Box<dyn std::error::Error>> {
    let mistral_api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");
    let client = Client::new();

    let request_body = serde_json::json!({
        "model": API_MODEL,
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
            if !response.status().is_success() {
                eprintln!("Client Error: {}", response.status());
            }

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

    let cache = open_file_cache().await;
    parse_mistral_stream(response, cache)
        .await
        .expect("Result stream chunking failed");

    Ok(())
}
