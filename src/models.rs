use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub role: Option<String>,
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChoice {
    pub index: u32,
    pub finish_reason: Option<String>,
    pub delta: Delta,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub id: String,
    pub created: u64,
    pub model: String,
    pub object: Option<String>,
    pub usage: Option<String>,
    pub p: Option<String>,
    pub choices: Vec<ChatCompletionChoice>,
}

#[derive(Debug, Deserialize)]
pub struct MistralModelResponse {
    pub data: Vec<MistralModelCard>,
    pub object: String,
}

#[derive(Debug, Deserialize)]
pub struct MistralModelCard {
    pub aliases: Vec<String>,
    pub archived: Option<bool>,
    pub capabilities: MistralModelCapabilities,
    pub created: i64,
    pub description: Option<String>,
    pub default_model_temperature: Option<f64>,
    pub deprecation: Option<String>, // datetime
    pub deprecation_replacement_model: Option<String>,
    pub id: String,
    pub job: Option<String>,
    pub max_context_length: i64,
    pub name: Option<String>,
    pub object: String,
    pub owned_by: String,
    pub r#type: String,
    pub root: Option<String>,
    pub max_content_length: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct MistralModelCapabilities {
    pub audio: bool,
    pub classification: bool,
    pub completion_chat: bool,
    pub completion_fim: bool,
    pub fine_tuning: bool,
    pub function_calling: bool,
    pub moderation: bool,
    pub ocr: bool,
    pub vision: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    use simd_json;

    #[tokio::test]
    async fn test_deserialize_chat_completion_chunk_with_bytes() {
        let json_bytes = br#"
            {
                "id": "cb22603a2ec344e8bd5db22c974bbe0f",
                "object": "chat.completion.chunk",
                "created": 1765372763,
                "model": "mistral-tiny",
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "content": "ku"
                        },
                        "finish_reason": null
                    }
                ],
                "p": "abcdefg"
            }
        "#;

        let mut buffer = Vec::new();
        buffer.extend_from_slice(json_bytes);

        let chunk: ChatCompletionChunk = simd_json::from_slice(&mut buffer).unwrap();

        // Verify the deserialized fields
        assert_eq!(chunk.id, "cb22603a2ec344e8bd5db22c974bbe0f");
        assert_eq!(chunk.created, 1765372763);
        assert_eq!(chunk.model, "mistral-tiny");
        assert_eq!(chunk.object, Some("chat.completion.chunk".to_string()));
        assert_eq!(chunk.usage, None);
        assert_eq!(chunk.p, Some("abcdefg".to_string()));
        assert_eq!(chunk.choices.len(), 1);

        // Verify the choice fields
        let choice = &chunk.choices[0];
        assert_eq!(choice.index, 0);
        assert_eq!(choice.finish_reason, None);
        assert_eq!(choice.delta.role, None);
        assert_eq!(choice.delta.content, Some("ku".to_string()));
    }

    #[tokio::test]
    async fn test_deserialize_chat_completion_chunk_without_p_bytes() {
        let json_bytes: &[u8] = br#"
            {
                "id": "cb22603a2ec344e8bd5db22c974bbe0f",
                "object": "chat.completion.chunk",
                "created": 1765372763,
                "model": "mistral-tiny",
                "choices": [
                    {
                        "index": 0,
                        "delta": {
                            "content": "ku"
                        },
                        "finish_reason": null
                    }
                ]
            }
        "#;

        let mut buffer = Vec::new();
        buffer.extend_from_slice(json_bytes);

        let chunk = simd_json::from_slice::<ChatCompletionChunk>(&mut buffer).unwrap();

        // Verify the deserialized fields
        assert_eq!(chunk.id, "cb22603a2ec344e8bd5db22c974bbe0f");
        assert_eq!(chunk.created, 1765372763);
        assert_eq!(chunk.model, "mistral-tiny");
        assert_eq!(chunk.object, Some("chat.completion.chunk".to_string()));
        assert_eq!(chunk.usage, None);
        assert_eq!(chunk.p, None); // `p` should be `None` when not present in the JSON
        assert_eq!(chunk.choices.len(), 1);

        // Verify the choice fields
        let choice = &chunk.choices[0];
        assert_eq!(choice.index, 0);
        assert_eq!(choice.finish_reason, None);
        assert_eq!(choice.delta.role, None);
        assert_eq!(choice.delta.content, Some("ku".to_string()));
    }
}
