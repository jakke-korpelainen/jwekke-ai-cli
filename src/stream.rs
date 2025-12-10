use crate::models::ChatCompletionChunk;
use bytes::Bytes;
use tokio::sync::mpsc;
use tokio_stream::StreamExt;

const STREAM_EOS: &'static str = "[DONE]";

// removes data: prefix and trims whitespace
fn clean_byte_chunk(buffer: &[u8]) -> Vec<u8> {
    let s = String::from_utf8_lossy(buffer);
    let cleaned = s
        .trim_start()
        .strip_prefix("data: ")
        .unwrap_or(&s)
        .trim_end();

    if cleaned == STREAM_EOS {
        return Vec::new();
    }

    cleaned.as_bytes().to_vec()
}

enum StreamState {
    FirstChunk,
    PartialChunk,
    SubsequentChunk,
}

impl std::fmt::Display for StreamState {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StreamState::FirstChunk => write!(f, "FirstChunk"),
            StreamState::PartialChunk => write!(f, "PartialChunk"),
            StreamState::SubsequentChunk => write!(f, "SubsequentChunk"),
        }
    }
}

pub async fn parse_mistral_stream(
    mut stream: futures::stream::BoxStream<'static, Result<Bytes, reqwest::Error>>,
    sender: mpsc::Sender<String>,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut parsed_content = String::new();
    println!("");

    let mut buffer = Vec::new();
    let mut stream_state = StreamState::FirstChunk;

    while let Some(chunk) = stream.next().await {
        if let Ok(bytes) = chunk {
            let cleaned_bytes = clean_byte_chunk(&bytes);
            if cleaned_bytes.is_empty() {
                continue;
            }

            let mut buffer_temp = buffer.clone();
            buffer_temp.extend_from_slice(&clean_byte_chunk(&cleaned_bytes));

            let json = simd_json::from_slice::<ChatCompletionChunk>(&mut buffer_temp);
            match json {
                Ok(value) => {
                    if let Some(content) = &value.choices[0].delta.content {
                        parsed_content.push_str(content);
                        sender
                            .send(content.clone())
                            .await
                            .expect("Failed to send chunk");
                    }
                    buffer.clear();
                }
                Err(e) => {
                    let current_buffer_text = String::from_utf8_lossy(&buffer);
                    let incoming_chunk_text = String::from_utf8_lossy(&cleaned_bytes);
                    match stream_state {
                        StreamState::FirstChunk if !incoming_chunk_text.starts_with("{") => {
                            //eprintln!("Token stream starts broken, can't stitch");
                        }
                        StreamState::PartialChunk if current_buffer_text.contains("}") => {
                            //eprintln!("Can't stitch, buffer already contains a complete chunk\n");
                            //eprintln!("{}", current_buffer_text);
                            panic!();
                        }
                        StreamState::PartialChunk | StreamState::SubsequentChunk
                            if !incoming_chunk_text.ends_with("}") =>
                        {
                            let mut stitched_buffer = Vec::new();
                            stitched_buffer.extend_from_slice(&buffer);
                            stitched_buffer.extend_from_slice(&cleaned_bytes);
                            buffer.extend_from_slice(&stitched_buffer);
                            stream_state = StreamState::PartialChunk;
                            continue;
                        }
                        _ => {
                            //println!("State: {}", stream_state);
                            //println!("Broken buffer: {}", incoming_chunk_text.escape_debug());
                            //eprintln!("^----- Error {} occurred while parsing buffer\n", e);
                        }
                    }
                }
            }
        }
        stream_state = StreamState::SubsequentChunk;
    }

    Ok(parsed_content)
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn test_parse_mistral_stream(
        specimen: Vec<String>,
    ) -> Result<String, Box<dyn std::error::Error>> {
        let stream = futures::stream::iter(specimen.into_iter().map(|s| Ok(Bytes::from(s))));
        let (sender, mut receiver) = mpsc::channel(100);

        // Spawn a task to consume the channel
        let _ = tokio::spawn(async move {
            while let Some(_) = receiver.recv().await {
                // Consume the chunks to prevent the channel from filling up
            }
        });

        parse_mistral_stream(Box::pin(stream), sender).await
    }

    #[tokio::test]
    async fn stream_test_specimen_1() {
        let specimen_vect_stream_text: Vec<String> = vec![
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Tur\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqr\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ku\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopq\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" River\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmn\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"ab\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Finland\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwx\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\":\"},\"finish_reason\":null}],\"p\":\"abcd\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Historical\"},\"finish_reason\":null}],\"p\":\"abcdefghij\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" water\"},\"finish_reason\":null}],\"p\":\"abcd\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"way\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxy\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" connecting\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvw\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" the\"},\"finish_reason\":null}],\"p\":\"abcdefgh\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Balt\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxy\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ic\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvw\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Sea\"},\"finish_reason\":null}],\"p\":\"abcde\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" with\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" a\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0123456\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" vibr\"},\"finish_reason\":null}],\"p\":\"abcdefghi\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ant\"},\"finish_reason\":null}],\"p\":\"abcdefghijk\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" city\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvw\"}".to_string(),
            "data: {\"id\":\"ad4cfdaf3a3045bd807b8c37c6199077\",\"object\":\"chat.completion.chunk\",\"created\":1765374050,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" known\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz\"}".to_string(),
            "data: [DONE]".to_string(),
        ];

        let result = test_parse_mistral_stream(specimen_vect_stream_text)
            .await
            .unwrap();
        assert_eq!(
            result,
            "Turku River, Finland: Historical waterway connecting the Baltic Sea with a vibrant city known"
        );
    }

    #[tokio::test]
    async fn stream_test_specimen_2() {
        let specimen_vect_stream_text: Vec<String> = vec![
            "data: {\"id\":\"3e59fe3d4d3c4d4aa365ef7b960e8927\",\"object\":\"chat.completion.chunk\",\"created\":1765376243,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ku\"},\"finish_reason\":null}],\"p\":\"abcdefghij\"}".to_string(),
        ];
        let result = test_parse_mistral_stream(specimen_vect_stream_text)
            .await
            .unwrap();
        assert_eq!(result, "ku");
    }
}
