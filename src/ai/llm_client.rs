use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::{Result, Context as AnyhowContext};

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct LLMClient {
    client: Client,
    endpoint: String,
    model: String,
}

impl LLMClient {
    pub fn new(endpoint: &str, model: &str) -> Self {
        Self {
            client: Client::new(),
            endpoint: endpoint.to_string(),
            model: model.to_string(),
        }
    }

    pub fn default() -> Self {
        let model = std::env::var("LST_MODEL").unwrap_or_else(|_| "qwen2.5:0.5b".to_string());
        Self::new("http://localhost:11434/api/generate", &model)
    }

    pub async fn generate_code(&self, prompt: &str) -> Result<String> {
        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
        };

        let res = self.client.post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API Error: {} - {}", status, text);
        }

        let body = res.text().await.context("Failed to read response text")?;

        let response: OllamaResponse = serde_json::from_str(&body)
            .context(format!("Failed to parse JSON body: {}", body))?;

        Ok(response.response)
    }
}
