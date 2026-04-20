// HuggingFace embedding provider.
//
// Two endpoint modes:
//   - Serverless (default): POST to
//     `{base_url}/{model}/pipeline/feature-extraction` against HF's
//     router (`https://router.huggingface.co/hf-inference/models`).
//   - Dedicated HF Inference Endpoint: when the caller overrides
//     `--embed-base-url`, POST directly to that URL and do NOT append
//     the model path (dedicated endpoints are model-locked).
//
// Body: `{"inputs": text}`. Auth: `Authorization: Bearer <HF_TOKEN>`.
//
// Response shape varies by deployment:
//   - `[[f32, ...]]` (nested — TEI / sentence-transformers default)
//   - `[f32, ...]`   (flat  — some dedicated deployments)
// The parser accepts both via `serde_json::Value` branching and returns
// the first vector.
//
// Error mapping per REQ-F-008 (exact strings asserted by tests):
//   - HTTP 401 / 403 ->
//       "huggingface auth failed: check HF_TOKEN scopes (needs Inference Providers permission)"
//   - HTTP 404       ->
//       "huggingface model '<m>' not found or not deployed by any Inference Provider. \
//        Try '<m>-multilingual-v1' or deploy a dedicated endpoint and set NEO4J_EMBED_BASE_URL"
//   - Other non-2xx   -> "huggingface <status>: <body>"  (wrapped as ProviderError)
//   - Connect/timeout -> "huggingface unreachable at <url>: <err>"
//   - Empty response  -> "empty response"

use async_trait::async_trait;
use reqwest::{Client, StatusCode};
use serde_json::{json, Value};

use super::{EmbedError, EmbedProvider};

const DEFAULT_BASE_URL: &str = "https://router.huggingface.co/hf-inference/models";

pub struct HuggingFace {
    client: Client,
    api_key: String,
    model: String,
    /// When `Some(_)`, treat as a dedicated Inference Endpoint URL and
    /// POST directly (no `/{model}/pipeline/feature-extraction` suffix).
    /// When `None`, use the HF router default base URL and append the
    /// serverless model path.
    base_url: Option<String>,
    // Accepted but ignored per REQ-F-007 (HF feature-extraction has no
    // variable-dim param). Kept for trait symmetry.
    #[allow(dead_code)]
    dimensions: Option<u32>,
}

impl HuggingFace {
    /// Construct a new HuggingFace provider.
    ///
    /// `api_key == None` -> MissingApiKey error (caller should have
    /// resolved `HF_TOKEN` / `NEO4J_EMBED_API_KEY` via `resolve_api_key`).
    pub fn new(
        api_key: Option<String>,
        model: String,
        dimensions: Option<u32>,
        base_url: Option<String>,
    ) -> Result<Self, EmbedError> {
        let api_key = api_key
            .filter(|k| !k.is_empty())
            .ok_or(EmbedError::MissingApiKey {
                provider: "huggingface",
                env_var: "HF_TOKEN",
            })?;

        // Empty-string base_url is treated as "not overridden" (matches
        // openai/ollama behaviour and dotenv stub-line ergonomics).
        let base_url = base_url.filter(|u| !u.is_empty());

        Ok(Self {
            client: Client::new(),
            api_key,
            model,
            base_url,
            dimensions,
        })
    }

    fn request_url(&self) -> String {
        match &self.base_url {
            Some(url) => url.clone(),
            None => format!(
                "{}/{}/pipeline/feature-extraction",
                DEFAULT_BASE_URL, self.model
            ),
        }
    }
}

#[async_trait]
impl EmbedProvider for HuggingFace {
    async fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError> {
        let url = self.request_url();
        let body = json!({ "inputs": input });

        let resp = match self
            .client
            .post(&url)
            .bearer_auth(&self.api_key)
            .json(&body)
            .send()
            .await
        {
            Ok(r) => r,
            Err(e) => {
                if e.is_connect() || e.is_request() || e.is_timeout() {
                    return Err(EmbedError::ProviderError {
                        provider: "huggingface",
                        message: format!("huggingface unreachable at {url}: {e}"),
                    });
                }
                return Err(EmbedError::Http(e));
            }
        };

        let status = resp.status();
        if !status.is_success() {
            if status == StatusCode::UNAUTHORIZED || status == StatusCode::FORBIDDEN {
                return Err(EmbedError::ProviderError {
                    provider: "huggingface",
                    message:
                        "huggingface auth failed: check HF_TOKEN scopes (needs Inference Providers permission)"
                            .to_string(),
                });
            }
            if status == StatusCode::NOT_FOUND {
                return Err(EmbedError::ProviderError {
                    provider: "huggingface",
                    message: format!(
                        "huggingface model '{model}' not found or not deployed by any Inference Provider. \
                         Try '{model}-multilingual-v1' or deploy a dedicated endpoint and set NEO4J_EMBED_BASE_URL",
                        model = self.model
                    ),
                });
            }
            let text = resp.text().await.unwrap_or_default();
            return Err(EmbedError::ProviderError {
                provider: "huggingface",
                message: format!("huggingface {status}: {text}"),
            });
        }

        let value: Value = resp.json().await?;
        parse_embedding(&value).ok_or_else(|| EmbedError::ProviderError {
            provider: "huggingface",
            message: "empty response".to_string(),
        })
    }

    fn model(&self) -> &str {
        &self.model
    }

    fn dimensions(&self) -> Option<u32> {
        self.dimensions
    }
}

/// Extract the first embedding vector from an HF feature-extraction
/// response. Handles both the nested `[[f32, ...]]` shape (TEI /
/// sentence-transformers default) and the flat `[f32, ...]` shape that
/// some dedicated deployments return.
fn parse_embedding(value: &Value) -> Option<Vec<f32>> {
    let arr = value.as_array()?;
    let first = arr.first()?;

    // Nested: [[f32, ...], ...] -> first inner array is the vector.
    if let Some(inner) = first.as_array() {
        return inner.iter().map(|v| v.as_f64().map(|f| f as f32)).collect();
    }

    // Flat: [f32, ...] -> the whole array is the vector.
    if first.as_f64().is_some() {
        return arr.iter().map(|v| v.as_f64().map(|f| f as f32)).collect();
    }

    None
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn parses_nested_shape() {
        let v = json!([[0.1, 0.2, 0.3]]);
        assert_eq!(parse_embedding(&v), Some(vec![0.1_f32, 0.2, 0.3]));
    }

    #[test]
    fn parses_flat_shape() {
        let v = json!([0.1, 0.2, 0.3]);
        assert_eq!(parse_embedding(&v), Some(vec![0.1_f32, 0.2, 0.3]));
    }

    #[test]
    fn rejects_non_array() {
        let v = json!({"oops": "wrong"});
        assert_eq!(parse_embedding(&v), None);
    }

    #[test]
    fn rejects_empty_array() {
        let v = json!([]);
        assert_eq!(parse_embedding(&v), None);
    }

    #[test]
    fn request_url_serverless_appends_model_path() {
        let hf = HuggingFace::new(
            Some("tok".to_string()),
            "sentence-transformers/foo".to_string(),
            None,
            None,
        )
        .unwrap();
        assert_eq!(
            hf.request_url(),
            "https://router.huggingface.co/hf-inference/models/sentence-transformers/foo/pipeline/feature-extraction"
        );
    }

    #[test]
    fn request_url_dedicated_uses_base_url_verbatim() {
        let hf = HuggingFace::new(
            Some("tok".to_string()),
            "ignored".to_string(),
            None,
            Some("https://example.endpoints.huggingface.cloud".to_string()),
        )
        .unwrap();
        assert_eq!(
            hf.request_url(),
            "https://example.endpoints.huggingface.cloud"
        );
    }

    #[test]
    fn missing_api_key_surfaces_exact_error() {
        let err = HuggingFace::new(None, "m".to_string(), None, None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "missing API key for huggingface: set HF_TOKEN"
        );
    }

    #[test]
    fn empty_api_key_treated_as_missing() {
        let err = HuggingFace::new(Some(String::new()), "m".to_string(), None, None).unwrap_err();
        assert_eq!(
            err.to_string(),
            "missing API key for huggingface: set HF_TOKEN"
        );
    }
}
