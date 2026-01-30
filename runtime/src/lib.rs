use reqwest::Client;
use serde_json::json;
use xnet_core::{DynError, RuntimeInterface};
use async_trait::async_trait;

#[derive(Clone)]
pub struct OllamaRuntime {
    client: Client,
    base_url: String,
}

impl OllamaRuntime {
    pub fn new(base_url: impl Into<String>) -> Self {
        Self {
            client: Client::new(),
            base_url: base_url.into(),
        }
    }

    pub async fn list_models(&self) -> Result<Vec<String>, DynError> {
        let url = format!("{}/api/tags", self.base_url);
        let res = self.client.get(&url).send().await?;
        
        if !res.status().is_success() {
            return Err(format!("Ollama API error: {}", res.status()).into());
        }

        let payload: serde_json::Value = res.json().await?;
        let models = payload["models"]
            .as_array()
            .ok_or("Invalid models response")?
            .iter()
            .filter_map(|m| m["name"].as_str().map(String::from))
            .collect();

        Ok(models)
    }
}

#[async_trait]
impl RuntimeInterface for OllamaRuntime {
    async fn generate(&self, model: &str, prompt: &str) -> Result<String, DynError> {
        let url = format!("{}/api/generate", self.base_url);
        let body = json!({
            "model": model,
            "prompt": prompt,
            "stream": false
        });

        let res = self.client.post(&url)
            .json(&body)
            .send()
            .await?;

        if !res.status().is_success() {
             return Err(format!("Ollama API error: {}", res.status()).into());
        }

        let payload: serde_json::Value = res.json().await?;
        let response_text = payload["response"].as_str()
            .ok_or("Invalid response format")?
            .to_string();

        Ok(response_text)
    }
}
