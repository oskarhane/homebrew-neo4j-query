// Embedding provider module.
//
// Defines the `EmbedProvider` trait, `EmbedError`, `EmbedCliArgs`, and
// `EmbedConfig` factory. Concrete providers live in sibling submodules
// (`openai`, `ollama`) and are wired up in later tasks.

#[allow(dead_code)]
pub mod ollama;
#[allow(dead_code)]
pub mod openai;

use async_trait::async_trait;
use clap::Args;
use thiserror::Error;

/// Async trait for embedding providers.
///
/// Implementors must be `Send + Sync` so they can be used behind a
/// `Box<dyn EmbedProvider>` across await points in the tokio runtime.
#[allow(dead_code)]
#[async_trait]
pub trait EmbedProvider: Send + Sync {
    /// Embed a single input string into a vector of floats.
    async fn embed(&self, input: &str) -> Result<Vec<f32>, EmbedError>;

    /// Name of the model in use (e.g. `"text-embedding-3-small"`).
    fn model(&self) -> &str;

    /// Optional explicit output dimensions. `None` means the provider's
    /// default for the configured model.
    fn dimensions(&self) -> Option<u32>;
}

/// Errors surfaced by embedding providers and config resolution.
///
/// Messages intentionally match REQ-F-011 exactly; downstream code asserts
/// on `format!("{err}")` so do not change these strings without updating
/// tests and docs.
#[allow(dead_code)]
#[derive(Debug, Error)]
pub enum EmbedError {
    #[error("missing API key for {provider}: set {env_var}")]
    MissingApiKey {
        provider: &'static str,
        env_var: &'static str,
    },

    #[error("embedding provider not configured: set NEO4J_EMBED_PROVIDER")]
    NotConfigured,

    #[error("NEO4J_EMBED_MODEL not set")]
    ModelNotSet,

    #[error("unknown provider: {0}")]
    UnknownProvider(String),

    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("{provider} error: {message}")]
    ProviderError {
        provider: &'static str,
        message: String,
    },
}

/// Clap-flattenable embed CLI flags.
///
/// Flattened onto both `QueryArgs` (for `-P v:embed=...` resolution in
/// query mode) and the `embed` subcommand. Every flag is backed by its
/// `NEO4J_EMBED_*` environment variable via clap's `env` attribute, so
/// precedence is CLI flag > shell env > `.env` (loaded by `load_env()`
/// before clap parses).
#[allow(dead_code)]
#[derive(Args, Debug, Clone, Default)]
pub struct EmbedCliArgs {
    /// Embedding provider name (`openai` or `ollama`).
    #[arg(long = "embed-provider", env = "NEO4J_EMBED_PROVIDER")]
    pub provider: Option<String>,

    /// Embedding model name (e.g. `text-embedding-3-small`, `all-minilm`).
    #[arg(long = "embed-model", env = "NEO4J_EMBED_MODEL")]
    pub model: Option<String>,

    /// Explicit output dimensions (OpenAI only; Ollama ignores).
    #[arg(long = "embed-dimensions", env = "NEO4J_EMBED_DIMENSIONS")]
    pub dimensions: Option<u32>,

    /// Override provider base URL.
    #[arg(long = "embed-base-url", env = "NEO4J_EMBED_BASE_URL")]
    pub base_url: Option<String>,
}

/// Resolved embed configuration, ready to hand to a provider factory.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct EmbedConfig {
    pub provider: String,
    pub model: String,
    pub dimensions: Option<u32>,
    pub base_url: Option<String>,
    pub api_key: Option<String>,
}

impl EmbedConfig {
    /// Resolve an `EmbedConfig` from clap args + environment.
    ///
    /// Returns `Ok(None)` when no provider is configured (query mode may
    /// still succeed without embeddings). Returns `Err(NotConfigured)`
    /// only when the caller explicitly needs a provider — see
    /// `require_provider` below. Returns `Err(...)` when a provider IS
    /// set but required fields (model) are missing, so config errors
    /// surface eagerly rather than as a generic "model not set" at
    /// request time.
    #[allow(dead_code)]
    pub fn from_sources(args: &EmbedCliArgs) -> Result<Option<Self>, EmbedError> {
        let provider = match args.provider.as_deref() {
            Some(p) if !p.is_empty() => p.to_string(),
            _ => return Ok(None),
        };

        let model = args
            .model
            .as_deref()
            .filter(|m| !m.is_empty())
            .ok_or(EmbedError::ModelNotSet)?
            .to_string();

        let api_key = resolve_api_key(&provider);

        Ok(Some(Self {
            provider,
            model,
            dimensions: args.dimensions,
            base_url: args.base_url.clone(),
            api_key,
        }))
    }

    /// Same as `from_sources` but treats missing provider as an error.
    /// Call this from paths that NEED a provider (e.g. `:embed` param
    /// resolution, `embed` subcommand execution).
    #[allow(dead_code)]
    pub fn require(args: &EmbedCliArgs) -> Result<Self, EmbedError> {
        Self::from_sources(args)?.ok_or(EmbedError::NotConfigured)
    }

    /// Build the concrete `EmbedProvider` for this config. Providers are
    /// implemented in task-005 (`openai`) and task-006 (`ollama`); this
    /// function currently errors for any provider name other than a
    /// stubbed match so callers can be wired up before providers exist.
    #[allow(dead_code)]
    pub fn build(self) -> Result<Box<dyn EmbedProvider>, EmbedError> {
        match self.provider.as_str() {
            "openai" => {
                let provider =
                    openai::OpenAi::new(self.api_key, self.model, self.dimensions, self.base_url)?;
                Ok(Box::new(provider))
            }
            "ollama" => Err(EmbedError::ProviderError {
                provider: "ollama",
                message: "ollama provider not yet implemented".to_string(),
            }),
            other => Err(EmbedError::UnknownProvider(other.to_string())),
        }
    }
}

/// Resolve API key for a given provider name.
///
/// - `openai`: `OPENAI_API_KEY` wins, then `NEO4J_EMBED_API_KEY` fallback.
/// - `ollama`: always `None`; `NEO4J_EMBED_API_KEY` is silently ignored
///   per REQ-F-006.
/// - anything else: `NEO4J_EMBED_API_KEY` if present.
#[allow(dead_code)]
pub fn resolve_api_key(provider: &str) -> Option<String> {
    match provider {
        "openai" => std::env::var("OPENAI_API_KEY")
            .ok()
            .filter(|v| !v.is_empty())
            .or_else(|| {
                std::env::var("NEO4J_EMBED_API_KEY")
                    .ok()
                    .filter(|v| !v.is_empty())
            }),
        "ollama" => None,
        _ => std::env::var("NEO4J_EMBED_API_KEY")
            .ok()
            .filter(|v| !v.is_empty()),
    }
}
