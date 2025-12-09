use crate::models::ChatCompletionChunk;
use std::{io::Write, path::PathBuf};
use tokio_stream::StreamExt;

const STREAM_EOS: &'static str = "[DONE]";

pub fn deserialize_sse_events(chunk: &str) -> Vec<String> {
    let mut events = Vec::new();
    let lines: Vec<&str> = chunk.lines().collect();
    let mut buffer = String::new();

    for line in lines {
        if line.starts_with("data: ") {
            let json_str = line.trim_start_matches("data: ").trim();
            if json_str != STREAM_EOS && !json_str.is_empty() {
                buffer.push_str(json_str);
                match serde_json::from_str::<serde_json::Value>(&buffer) {
                    Ok(_) => {
                        events.push(buffer.clone());
                        buffer.clear();
                    }
                    Err(e) => {
                        // Only push the error if it's not an EOF error
                        if !e.is_eof() {
                            eprintln!("Error {} occurred while buffering chunk: {}", e, chunk);
                            buffer.clear(); // Clear buffer on non-EOF error
                        }
                    }
                }
            }
        }
    }

    events
}

pub async fn parse_mistral_stream(
    response: reqwest::Response,
    (_, mut file): (PathBuf, std::fs::File),
) -> Result<(), Box<dyn std::error::Error>> {
    let mut stream = response.bytes_stream();

    while let Some(chunk) = stream.next().await {
        let chunk = chunk?;
        let chunk_str = String::from_utf8_lossy(&chunk);
        let events = deserialize_sse_events(&chunk_str);

        for event in events {
            match serde_json::from_str::<ChatCompletionChunk>(&event) {
                Ok(chunk) => {
                    if let Some(content) = &chunk.choices[0].delta.content {
                        let value = content.to_string();
                        // Write the content to the file
                        file.write_all(value.as_bytes())?;
                        file.flush()?;
                    }
                }
                Err(e) => {
                    eprintln!("Error {} occurred while parsing event {}", e, event);
                }
            }
        }
    }

    // Ensure the child process is properly terminated
    drop(file); // Close stdin to signal EOF to the child process

    Ok(())
}
