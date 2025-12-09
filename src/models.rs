use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Delta {
    pub content: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct Choice {
    pub delta: Delta,
}

#[derive(Debug, Deserialize)]
pub struct ChatCompletionChunk {
    pub choices: Vec<Choice>,
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
