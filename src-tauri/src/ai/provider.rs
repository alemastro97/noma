//! Where completions come from: the bundled llama.cpp server (default) or a
//! local Ollama (developer convenience, never required).

use std::time::Duration;

use serde_json::{Value, json};

use super::AiError;

#[derive(Debug, Clone)]
pub enum Provider {
    /// OpenAI-compatible llama-server on localhost.
    Llama {
        base_url: String,
        thinking: bool,
    },
    Ollama {
        base_url: String,
        model: String,
    },
}

impl Provider {
    pub fn label(&self) -> String {
        match self {
            Provider::Llama { .. } => "built-in".into(),
            Provider::Ollama { model, .. } => format!("Ollama · {model}"),
        }
    }

    /// One schema-constrained JSON completion. Temperature 0 for repeatable parses.
    pub async fn complete_json(
        &self,
        client: &reqwest::Client,
        system: &str,
        user: &str,
        schema: &Value,
    ) -> Result<String, AiError> {
        let messages = json!([
            { "role": "system", "content": system },
            { "role": "user", "content": user }
        ]);
        match self {
            Provider::Llama { base_url, thinking } => {
                let mut body = json!({
                    "messages": messages,
                    "temperature": 0,
                    "max_tokens": 700,
                    "cache_prompt": true,
                    "response_format": { "type": "json_schema", "schema": schema },
                });
                if *thinking {
                    body["chat_template_kwargs"] = json!({ "enable_thinking": false });
                }
                let resp: Value = client
                    .post(format!("{base_url}/v1/chat/completions"))
                    .timeout(Duration::from_secs(60))
                    .json(&body)
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                resp["choices"][0]["message"]["content"]
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| AiError::BadOutput("empty completion".into()))
            }
            Provider::Ollama { base_url, model } => {
                let body = json!({
                    "model": model,
                    "messages": messages,
                    "format": schema,
                    "stream": false,
                    "think": false,
                    "keep_alive": "10m",
                    "options": { "temperature": 0, "num_predict": 700 },
                });
                let resp: Value = client
                    .post(format!("{base_url}/api/chat"))
                    .timeout(Duration::from_secs(120))
                    .json(&body)
                    .send()
                    .await?
                    .error_for_status()?
                    .json()
                    .await?;
                resp["message"]["content"]
                    .as_str()
                    .map(str::to_string)
                    .ok_or_else(|| AiError::BadOutput("empty completion".into()))
            }
        }
    }
}

/// Model names a local Ollama has pulled, or `None` when it is not running.
pub async fn ollama_models(client: &reqwest::Client, base_url: &str) -> Option<Vec<String>> {
    let resp: Value = client
        .get(format!("{base_url}/api/tags"))
        .timeout(Duration::from_millis(800))
        .send()
        .await
        .ok()?
        .json()
        .await
        .ok()?;
    Some(
        resp["models"]
            .as_array()?
            .iter()
            .filter_map(|m| m["name"].as_str().map(str::to_string))
            .collect(),
    )
}
