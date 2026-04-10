use std::path::PathBuf;

pub struct Agent {
    pub name: &'static str,
    pub display_name: &'static str,
    /// Directory whose existence indicates the agent is installed
    detect_dir: &'static str,
    /// Directory where skill folders/symlinks are placed
    skills_dir: &'static str,
}

impl Agent {
    /// Resolve detect_dir to an absolute path, expanding ~ and $XDG_CONFIG_HOME.
    pub fn detect_path(&self) -> Option<PathBuf> {
        expand_path(self.detect_dir)
    }

    /// Resolve skills_dir to an absolute path, expanding ~ and $XDG_CONFIG_HOME.
    pub fn skills_path(&self) -> Option<PathBuf> {
        expand_path(self.skills_dir)
    }
}

fn home_dir() -> Option<PathBuf> {
    std::env::var("HOME").ok().map(PathBuf::from)
}

static AGENTS: &[Agent] = &[
    Agent {
        name: "claude-code",
        display_name: "Claude Code",
        detect_dir: "~/.claude",
        skills_dir: "~/.claude/skills",
    },
    Agent {
        name: "cursor",
        display_name: "Cursor",
        detect_dir: "~/.cursor",
        skills_dir: "~/.cursor/skills",
    },
    Agent {
        name: "windsurf",
        display_name: "Windsurf",
        detect_dir: "~/.codeium/windsurf",
        skills_dir: "~/.codeium/windsurf/skills",
    },
    Agent {
        name: "copilot",
        display_name: "Copilot",
        detect_dir: "~/.copilot",
        skills_dir: "~/.copilot/skills",
    },
    Agent {
        name: "gemini-cli",
        display_name: "Gemini CLI",
        detect_dir: "~/.gemini",
        skills_dir: "~/.gemini/skills",
    },
    Agent {
        name: "cline",
        display_name: "Cline",
        detect_dir: "~/.cline",
        skills_dir: "~/.agents/skills",
    },
    Agent {
        name: "roo",
        display_name: "Roo",
        detect_dir: "~/.roo",
        skills_dir: "~/.roo/skills",
    },
    Agent {
        name: "codex",
        display_name: "Codex",
        detect_dir: "~/.codex",
        skills_dir: "~/.codex/skills",
    },
    Agent {
        name: "pi",
        display_name: "Pi",
        detect_dir: "~/.pi/agent",
        skills_dir: "~/.pi/agent/skills",
    },
    Agent {
        name: "opencode",
        display_name: "OpenCode",
        detect_dir: "$XDG_CONFIG_HOME/opencode",
        skills_dir: "$XDG_CONFIG_HOME/opencode/skills",
    },
    Agent {
        name: "junie",
        display_name: "Junie",
        detect_dir: "~/.junie",
        skills_dir: "~/.junie/skills",
    },
];

/// Expand ~ to home dir and $XDG_CONFIG_HOME (defaulting to ~/.config).
fn expand_path(path: &str) -> Option<PathBuf> {
    let home = home_dir()?;
    if let Some(rest) = path.strip_prefix("~/") {
        Some(home.join(rest))
    } else if path == "~" {
        Some(home)
    } else if path.contains("$XDG_CONFIG_HOME") {
        let xdg = std::env::var("XDG_CONFIG_HOME")
            .ok()
            .filter(|v| !v.is_empty())
            .map(PathBuf::from)
            .unwrap_or_else(|| home.join(".config"));
        Some(PathBuf::from(
            path.replace("$XDG_CONFIG_HOME", &xdg.to_string_lossy()),
        ))
    } else {
        Some(PathBuf::from(path))
    }
}

/// Canonical directory where the embedded SKILL.md is written.
pub fn canonical_dir() -> Option<PathBuf> {
    let home = home_dir()?;
    Some(home.join(".local/share/neo4j-query/skills/neo4j-query"))
}

/// Return all known agents.
pub fn all_agents() -> &'static [Agent] {
    AGENTS
}

/// Detect which agents are present by checking if their detect_dir exists.
pub fn detect_agents() -> Vec<&'static Agent> {
    AGENTS
        .iter()
        .filter(|agent| {
            agent
                .detect_path()
                .map(|p| std::fs::metadata(&p).is_ok())
                .unwrap_or(false)
        })
        .collect()
}

/// Find an agent by name (case-insensitive).
pub fn find_agent(name: &str) -> Option<&'static Agent> {
    let lower = name.to_lowercase();
    AGENTS.iter().find(|a| a.name == lower)
}
