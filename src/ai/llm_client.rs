use serde::{Deserialize, Serialize};
use reqwest::Client;
use anyhow::{Result, Context as AnyhowContext};
use std::time::Duration;

#[derive(Serialize)]
struct OllamaOptions {
    #[serde(skip_serializing_if = "Option::is_none")]
    temperature: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_p: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    top_k: Option<i32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repeat_penalty: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    num_predict: Option<i32>,
}

#[derive(Serialize)]
struct OllamaRequest {
    model: String,
    prompt: String,
    stream: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    grammar: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    options: Option<OllamaOptions>,
}

#[derive(Deserialize)]
struct OllamaResponse {
    response: String,
}

pub struct LLMClient {
    client: Client,
    endpoint: String,
    model: String,
    verbose: bool,
}

impl LLMClient {
    pub fn new(endpoint: &str, model: &str) -> Self {
        let client = Client::builder()
            .timeout(Duration::from_secs(180))
            .build()
            .unwrap_or_else(|_| Client::new());

        Self {
            client,
            endpoint: endpoint.to_string(),
            model: model.to_string(),
            verbose: false,
        }
    }

    pub fn with_verbose(mut self, verbose: bool) -> Self {
        self.verbose = verbose;
        self
    }

    pub fn default() -> Self {
        let model = std::env::var("HUII_MODEL").unwrap_or_else(|_| "codellama:7b".to_string());
        Self::new("http://localhost:11434/api/generate", &model)
    }

    pub async fn generate_code(&self, prompt: &str, grammar: Option<&str>) -> Result<String> {
        let options = OllamaOptions {
            temperature: Some(0.2),
            top_p: Some(0.9),
            top_k: Some(40),
            repeat_penalty: Some(1.1),
            num_predict: Some(2048),
        };

        let request = OllamaRequest {
            model: self.model.clone(),
            prompt: prompt.to_string(),
            stream: false,
            grammar: grammar.map(|s| s.to_string()),
            options: Some(options),
        };

        let res = self.client.post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send request to Ollama. Is Ollama running? (try: ollama serve)")?;

        if !res.status().is_success() {
            let status = res.status();
            let text = res.text().await.unwrap_or_default();
            anyhow::bail!("Ollama API Error: {} - {}", status, text);
        }

        let body = res.text().await.context("Failed to read response text")?;

        if self.verbose {
            println!("[verbose] Ollama response length: {} chars", body.len());
        }

        let response: OllamaResponse = serde_json::from_str(&body)
            .context(format!("Failed to parse JSON body: {}", body))?;

        Ok(response.response)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ollama_request_with_options() {
        let req = OllamaRequest {
            model: "test-model".to_string(),
            prompt: "hello".to_string(),
            stream: false,
            grammar: Some("root ::= item+".to_string()),
            options: Some(OllamaOptions {
                temperature: Some(0.2),
                top_p: Some(0.9),
                top_k: Some(40),
                repeat_penalty: Some(1.1),
                num_predict: Some(2048),
            }),
        };

        let json = serde_json::to_string(&req).expect("Failed to serialize");
        assert!(json.contains("\"grammar\":\"root ::= item+\""));
        assert!(json.contains("\"model\":\"test-model\""));
        assert!(json.contains("\"temperature\":0.2"));
        assert!(json.contains("\"repeat_penalty\":1.1"));
    }

    #[test]
    fn test_ollama_request_no_grammar_no_options() {
        let req = OllamaRequest {
            model: "test-model".to_string(),
            prompt: "hello".to_string(),
            stream: false,
            grammar: None,
            options: None,
        };

        let json = serde_json::to_string(&req).expect("Failed to serialize");
        assert!(!json.contains("\"grammar\""));
        assert!(!json.contains("\"options\""));
        assert!(json.contains("\"model\":\"test-model\""));
    }
}
