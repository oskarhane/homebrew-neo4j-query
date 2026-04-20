// OpenAI embedding provider.
//
// POSTs to `{base_url}/embeddings` with bearer auth and body
// `{input, model, dimensions?}`. Default base URL `https://api.openai.com/v1`.
// Error mapping per REQ-F-011:
//   - `new()` with `api_key = None` -> MissingApiKey { openai, OPENAI_API_KEY }
//   - non-2xx response           -> ProviderError("HTTP <status>: <body>")
//   - empty data[] in response    -> ProviderError("empty response")

use async_trait::async_trait;
use reqwest::Client;
use serde::Deserialize;
use serde_json::json;

use super::{EmbedError, EmbedProvider};

const DEFAULT_BASE_URL: &str = "https://api.openai.com/v1";

pub struct OpenAi {
    client: Client,
    base_url: String,
    api_key: String,
    model: String,
    dimensions: Option<u32>,
}

#[derive(Deserialize)]
struct EmbeddingsResponse {
    data: Vec<EmbeddingItem>,
}

#[derive(Deserialize)]
struct EmbeddingItem {
    embedding: Vec<f32>,
}

impl OpenAi {
    /// Construct a new OpenAI provider.
    ///
    /// `api_key == None` -> MissingApiKey error (caller should have resolved
    /// `OPENAI_API_KEY` / `NEO4J_EMBED_API_KEY` via `resolve_api_key`).
    /// `base_url == None` -> `https://api.openai.com/v1`.
    pub fn new(
        api_key: Option<String>,
        model: String,
        dimensions: Option<u32>,
        base_url: Option<String>,
    ) -> Result<Self, EmbedError> {
        let api_key = api_key
            .filter(|k| !k.is_empty())
            .ok_or(EmbedError::MissingApiKey {
                provider: "openai",
                env_var: "OPENAI_API_KEY",
            })?;

        let base_url = base_url
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        Ok(Self {
            client: Client::new(),
            base_url,
            api_key,
            model,
            dimensions,
        })
    }
}

#[async_trait]
impl EmbedProvider for OpenAi {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError> {
        let url = format!("{}/embeddings", self.base_url.trim_end_matches('/'));

        let mut body = json!({
            "input": input,
            "model": self.model,
        });
        if let Some(dims) = self.dimensions {
            body["dimensions"] = json!(dims);
        }

        let resp = self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await?;

        let status = resp.status();
        if !status.is_success() {
            let text = resp.text().await.unwrap_or_default();
            return Err(EmbedError::ProviderError {
                provider: "openai",
                message: format!("HTTP {status}: {text}"),
            });
        }

        let parsed: EmbeddingsResponse = resp.json().await?;
        let first = parsed
            .data
            .into_iter()
            .next()
            .ok_or_else(|| EmbedError::ProviderError {
                provider: "openai",
                message: "empty response".to_string(),
            })?;
        Ok(first.embedding)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> Option<u32> {
        self.dimensions
    }
}
