// `neo4j-query embed` subcommand.
//
// Produces an embedding vector for a piece of text using the configured
// provider. Two output formats are supported per REQ-F-004:
//   - `json` (default): single-line JSON array of floats.
//   - `raw`: one float per line.
// TOON is intentionally not supported — the output is always a bare vector,
// not a row/column structure TOON is designed for.
//
// The text comes from the positional `TEXT` argument, or stdin when the
// positional is omitted (trimmed). Empty input after trimming is an error.

use std::io::{self, IsTerminal, Read};

use clap::{Args, ValueEnum};

use crate::embed::{EmbedCliArgs, EmbedConfig};

/// Output format for the embed subcommand.
///
/// `toon` is intentionally absent — a raw vector doesn't map to TOON's
/// row/column model and adding it would invite confusion about layout.
#[derive(Clone, Debug, ValueEnum)]
pub enum EmbedFormat {
    /// Single-line JSON array of floats (default).
    Json,
    /// One float per line.
    Raw,
}

#[derive(Args, Debug)]
pub struct EmbedCmd {
    /// Text to embed. If omitted, reads from stdin (trimmed).
    pub text: Option<String>,

    /// Output format.
    #[arg(long, value_enum, default_value = "json")]
    pub format: EmbedFormat,
}

/// Resolve the input text: prefer the positional arg, fall back to stdin.
///
/// Empty input after trimming is rejected so the provider never sees a
/// blank request (most providers error on empty input anyway).
fn resolve_text(arg: Option<String>) -> Result<String, String> {
    if let Some(t) = arg {
        let trimmed = t.trim().to_string();
        if trimmed.is_empty() {
            return Err("empty text: provide TEXT argument or pipe via stdin".into());
        }
        return Ok(trimmed);
    }
    if !io::stdin().is_terminal() {
        let mut buf = String::new();
        io::stdin()
            .read_to_string(&mut buf)
            .map_err(|e| format!("failed to read stdin: {e}"))?;
        let trimmed = buf.trim().to_string();
        if trimmed.is_empty() {
            return Err("empty text from stdin".into());
        }
        return Ok(trimmed);
    }
    Err(
        "no text provided. Usage: neo4j-query embed \"TEXT\" or echo \"TEXT\" | neo4j-query embed"
            .into(),
    )
}

pub async fn run(
    cmd: EmbedCmd,
    embed_args: &EmbedCliArgs,
) -> Result<(), Box<dyn std::error::Error>> {
    let text = resolve_text(cmd.text)?;
    // `require` surfaces NotConfigured when no provider is set — matches
    // REQ-F-011 "embedding provider not configured: set NEO4J_EMBED_PROVIDER".
    let config = EmbedConfig::require(embed_args)?;
    let provider = config.build()?;
    let vector = provider.embed(&text).await?;

    match cmd.format {
        EmbedFormat::Json => {
            println!("{}", serde_json::to_string(&vector)?);
        }
        EmbedFormat::Raw => {
            for f in &vector {
                println!("{f}");
            }
        }
    }
    Ok(())
}
