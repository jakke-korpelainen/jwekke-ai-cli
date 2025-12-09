use reqwest::Client;

use crate::{file::create_file_cache, stream::parse_mistral_stream};

const API_URL: &'static str = "https://api.mistral.ai/v1/chat/completions";
const API_MODEL: &'static str = "mistral-tiny";

pub async fn call_mistral_completions(prompt: String) -> Result<(), Box<dyn std::error::Error>> {
    let mistral_api_key = std::env::var("MISTRAL_API_KEY").expect("MISTRAL_API_KEY not set");

    let cache = create_file_cache();

    let client = Client::new();

    let request_body = serde_json::json!({
        "model": API_MODEL,
        "messages": [{"role": "user", "content": prompt}],
        "stream": true
    });

    let response = client
        .post(API_URL)
        .header("Content-Type", "application/json")
        .header("Authorization", format!("Bearer {}", mistral_api_key))
        .json(&request_body)
        .send()
        .await?;

    if !response.status().is_success() {
        eprintln!("Request failed with status: {}", response.status());
        return Ok(());
    }

    parse_mistral_stream(response, cache)
        .await
        .expect("Result stream chunking failed");

    Ok(())
}
