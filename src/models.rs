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
