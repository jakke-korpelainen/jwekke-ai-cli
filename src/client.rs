use crate::models::{MistralModelCard, MistralModelResponse};
use crate::{config, logger::Logger, stream};
use reqwest::Client;
use std::env;

use tokio::sync::mpsc;

const API_URL: &'static str = "https://api.mistral.ai/v1/chat/completions";
const DEFAULT_API_MODEL: &'static str = "mistral-tiny";

pub async fn list_mistral_models(
    logger: &Logger,
) -> Result<Vec<MistralModelCard>, Box<dyn std::error::Error>> {
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
                logger
                    .log_error(format!("Client Error: {}", response.status()))
                    .await;

                return Err(Box::new(std::io::Error::new(
                    std::io::ErrorKind::Other,
                    format!("Client Error: {}", response.status()),
                )));
            }

            response
                .json::<MistralModelResponse>()
                .await
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error>)?
                .data
        }
        Err(e) => {
            logger.log_error(format!("Client Error: {}", e)).await;

            return Err(Box::new(e));
        }
    };

    // filter only models with chat completion

    Ok(response
        .into_iter()
        .filter(|model| model.capabilities.completion_chat == true)
        .collect())
}

pub async fn call_mistral_completions(
    prompt: String,
    sender: mpsc::Sender<String>,
    logger: &Logger,
) -> Result<(), Box<dyn std::error::Error>> {
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
                logger
                    .log_error(format!("Client Error: {}", response.status()))
                    .await;
            }

            response
        }
        Err(e) => {
            logger.log_error(format!("Client Error: {}", e)).await;
            return Err(e.into());
        }
    };

    stream::parse_mistral_stream(Box::pin(response.bytes_stream()), sender, &logger)
        .await
        .expect("Result stream chunking failed");

    Ok(())
}
