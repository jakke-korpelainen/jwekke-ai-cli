use crate::{logger::Logger, models::ChatCompletionChunk};
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
    logger: &Logger,
) -> Result<String, Box<dyn std::error::Error>> {
    let mut parsed_content = String::new();
    let mut buffer = Vec::new();
    let mut stream_state = StreamState::FirstChunk;

    while let Some(chunk) = stream.next().await {
        if let Ok(bytes) = chunk {
            logger.write_stream_log(bytes.clone()).await;
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

                    // stop if finished
                    if value.choices[0].finish_reason.is_some() {
                        break;
                    }
                }
                Err(e) => {
                    let current_buffer_text = String::from_utf8_lossy(&buffer);
                    let incoming_chunk_text = String::from_utf8_lossy(&cleaned_bytes);
                    match stream_state {
                        StreamState::FirstChunk if !incoming_chunk_text.starts_with("{") => {
                            logger
                                .log_error(format!("Token stream starts broken, can't stitch"))
                                .await;
                        }
                        StreamState::PartialChunk if current_buffer_text.contains("}") => {
                            logger
                                .log_error(format!(
                                    "Can't stitch, buffer already contains a complete chunk\n{}",
                                    current_buffer_text
                                ))
                                .await;
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
                            logger
                                .log_error(format!(
                                    "State: {}\nBroken buffer: {}\nError: {}",
                                    stream_state,
                                    incoming_chunk_text.escape_debug(),
                                    e
                                ))
                                .await;
                        }
                    }
                }
            }
        }
        stream_state = StreamState::SubsequentChunk;
    }

    drop(sender);

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
        let logger = Logger::new().await;

        // Spawn a task to consume the channel
        let _ = tokio::spawn(async move {
            while let Some(_) = receiver.recv().await {
                // Consume the chunks to prevent the channel from filling up
            }
        });

        parse_mistral_stream(Box::pin(stream), sender, &logger).await
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

    #[tokio::test]
    async fn stream_test_specimen_stitched_start() {
        let specimen_vect_stream_text: Vec<String> = vec![
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"role\":\"assistant\",\"content\":\"\"},\"finish_reason\":null}]}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"The\"},\"finish_reason\":null}],\"p\":\"abcdefghi\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" most\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" popular\"},\"finish_reason\":null}],\"p\":\"abcdefghijklm\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" K\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmn\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"FC\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnop\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" meal\"},\"finish_reason\":null}],\"p\":\"abcdefghijkl\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" can\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" vary\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrs\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" depending\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwx\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" on\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuv\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" the\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" region\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuv\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" but\"},\"finish_reason\":null}],\"p\":\"abcdefghijk\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" in\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345678\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" the\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345678\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" United\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstu\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" States\"},\"finish_reason\":null}],\"p\":\"abcdefghijklm\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvw\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" the\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" \\\"\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0123456789\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"Original\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxy\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Rec\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmno\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ipe\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0123\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" B\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0123456789\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ucket\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstu\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Me\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01234\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"al\"},\"finish_reason\":null}],\"p\":\"abcdefghijk\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\\\"\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxy\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" is\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwx\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" generally\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" the\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopq\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" best\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxy\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"-\"},\"finish_reason\":null}],\"p\":\"abcdefghijklm\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"selling\"},\"finish_reason\":null}],\"p\":\"abcdefghijklm\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" item\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqr\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\".\"},\"finish_reason\":null}],\"p\":\"a\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" This\"},\"finish_reason\":null}],\"p\":\"abcdefghijkl\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" meal\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrs\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" typically\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrs\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" includes\"},\"finish_reason\":null}],\"p\":\"abcdefgh\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" a\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwx\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" bucket\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvw\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" of\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Original\"},\"finish_reason\":null}],\"p\":\"abcdefghijkl\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" Rec\"},\"finish_reason\":null}],\"p\":\"abcdefghijklm\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ipe\"},\"finish_reason\":null}],\"p\":\"abcdefghi\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" f\"},\"finish_reason\":null}],\"p\":\"abcdefghijkl\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ried\"},\"finish_reason\":null}],\"p\":\"a\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" chicken\"},\"finish_reason\":null}],\"p\":\"abcdef\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abcdefghijk\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" a\"},\"finish_reason\":null}],\"p\":\"abcdefgh\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" variety\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01234567\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" of\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopq\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" side\"},\"finish_reason\":null}],\"p\":\"abcdefghij\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" options\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" such\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmn\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" as\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrs\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" mas\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01234\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"hed\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvw\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" potatoes\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0123\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" col\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmn\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"es\"},\"finish_reason\":null}],\"p\":\"abcdefghijkl\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"law\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmno\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abc\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" or\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" bis\"},\"finish_reason\":null}],\"p\":\"abcdefghijk\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"cu\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01234567\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"its\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01234567\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0123456789\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" and\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopq\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" a\"},\"finish_reason\":null}],\"p\":\"abcdefg\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" choice\"},\"finish_reason\":null}],\"p\":\"abc\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" of\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" gr\"},\"finish_reason\":null}],\"p\":\"abcdefghi\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"avy\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnop\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" or\"},\"finish_reason\":null}],\"p\":\"abcdefghijklm\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" cor\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnop\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"nb\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrs\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"read\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrst\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\".\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnop\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" However\"},\"finish_reason\":null}],\"p\":\"abcdefgh\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345678\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" it\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvw\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"'\"},\"finish_reason\":null}],\"p\":\"abcdefgh\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"s\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" always\"},\"finish_reason\":null}],\"p\":\"abcdefg\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" a\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwx\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" good\"},\"finish_reason\":null}],\"p\":\"abcdefghi\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" idea\"},\"finish_reason\":null}],\"p\":\"abcdefghi\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" to\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmno\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" check\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuv\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" with\"},\"finish_reason\":null}],\"p\":\"a\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" your\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopq\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" local\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" K\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmno\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"FC\"},\"finish_reason\":null}],\"p\":\"a\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\",\"},\"finish_reason\":null}],\"p\":\"abcdefgh\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" as\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz012345678\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" menu\"},\"finish_reason\":null}],\"p\":\"abcdefghijklm\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" offer\"},\"finish_reason\":null}],\"p\":\"a\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"ings\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz0\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" and\"},\"finish_reason\":null}],\"p\":\"abcdefghijkl\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" popularity\"},\"finish_reason\":null}],\"p\":\"abcdefghij\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" may\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwx\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\" differ\"},\"finish_reason\":null}],\"p\":\"abcdefghijklmnopqrstuvwxyz01234567\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\".\"},\"finish_reason\":null}],\"p\":\"abcdefghij\"}".to_string(),
            "data: {\"id\":\"ddbbe5d63c1a4187ae445c4c2a61afbe\",\"object\":\"chat.completion.chunk\",\"created\":1765487429,\"model\":\"mistral-tiny\",\"choices\":[{\"index\":0,\"delta\":{\"content\":\"\"},\"finish_reason\":\"stop\"},\"usage\":{\"prompt_tokens\":12,\"total_tokens\":121,\"completion_tokens\":109},\"p\":\"abcdefghijklmnopqrstuv\"}".to_string(),
            "data: [DONE]".to_string(),
        ];

        let result = test_parse_mistral_stream(specimen_vect_stream_text)
            .await
            .unwrap();
        assert_eq!(
            result,
            "The most popular KFC meal can vary depending on the region, but in the United States, the \"Original Recipe Bucket Meal\" is generally the best-selling item. This meal typically includes a bucket of Original Recipe fried chicken, a variety of side options such as mashed potatoes, coleslaw, or biscuits, and a choice of gravy or cornbread. However, it\'s always a good idea to check with your local KFC, as menu offerings and popularity may differ."
        );
    }
}
