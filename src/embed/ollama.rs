// Ollama embedding provider.
//
// POSTs to `{base_url}/api/embed` (plural shape) with body `{model, input}`.
// Default base URL `http://localhost:11434`. No API key is sent; if
// `NEO4J_EMBED_API_KEY` is set in the environment it is silently ignored
// (filtered out upstream by `resolve_api_key`, per REQ-F-006).
//
// Error mapping per REQ-F-010 / REQ-F-011:
//   - connection refused / unreachable ->
//       ProviderError("ollama unreachable at <url> (<cause>). is `ollama serve` running?")
//   - HTTP 404                        ->
//       ProviderError("model '<model>' not found. run: ollama pull <model>")
//   - other non-2xx                    ->
//       ProviderError("HTTP <status>: <body>")
//   - empty embeddings array          ->
//       ProviderError("empty response")

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde::Deserialize;
use serde_json::json;

use super::{EmbedError, EmbedProvider};

const DEFAULT_BASE_URL: &str = "http://localhost:11434";

#[allow(dead_code)]
pub struct Ollama {
    client: Client,
    base_url: String,
    model: String,
    dimensions: Option<u32>,
}

#[derive(Deserialize)]
struct EmbedResponse {
    embeddings: Vec<Vec<f32>>,
}

impl Ollama {
    /// Construct a new Ollama provider.
    ///
    /// No API key is accepted or required. `base_url == None` falls back to
    /// `http://localhost:11434`.
    #[allow(dead_code)]
    pub fn new(
        model: String,
        dimensions: Option<u32>,
        base_url: Option<String>,
    ) -> Result<Self, EmbedError> {
        let base_url = base_url
            .filter(|u| !u.is_empty())
            .unwrap_or_else(|| DEFAULT_BASE_URL.to_string());

        Ok(Self {
            client: Client::new(),
            base_url,
            model,
            dimensions,
        })
    }
}

#[async_trait]
impl EmbedProvider for Ollama {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError> {
        let url = format!("{}/api/embed", self.base_url.trim_end_matches('/'));

        let body = json!({
            "model": self.model,
            "input": input,
        });

        let resp = match self.client.post(&url).json(&body).send().await {
            Ok(r) => r,
            Err(e) => {
                // Treat connect / request-send failures (no response
                // received) as "ollama not running" — the overwhelmingly
                // common local cause. reqwest::Error exposes `is_connect()`
                // and `is_request()`; either indicates we never got a
                // response back.
                if e.is_connect() || e.is_request() || e.is_timeout() {
                    return Err(EmbedError::ProviderError {
                        provider: "ollama",
                        message: format!(
                            "ollama unreachable at {} ({e}). is `ollama serve` running?",
                            self.base_url
                        ),
                    });
                }
                return Err(EmbedError::Http(e));
            }
        };

        let status = resp.status();
        if !status.is_success() {
            if status == StatusCode::NOT_FOUND {
                return Err(EmbedError::ProviderError {
                    provider: "ollama",
                    message: format!(
                        "model '{}' not found. run: ollama pull {}",
                        self.model, self.model
                    ),
                });
            }
            let text = resp.text().await.unwrap_or_default();
            return Err(EmbedError::ProviderError {
                provider: "ollama",
                message: format!("HTTP {status}: {text}"),
            });
        }

        let parsed: EmbedResponse = resp.json().await?;
        let first =
            parsed
                .embeddings
                .into_iter()
                .next()
                .ok_or_else(|| EmbedError::ProviderError {
                    provider: "ollama",
                    message: "empty response".to_string(),
                })?;
        Ok(first)
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> Option<u32> {
        self.dimensions
    }
}
