use std::path::{Path, PathBuf};

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

/// Embedded SKILL.md content, baked in at compile time.
const SKILL_MD: &str = include_str!("../skills/neo4j-query/SKILL.md");

/// Recursively copy a directory.
fn copy_dir(src: &Path, dst: &Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let target = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir(&entry.path(), &target)?;
        } else {
            std::fs::copy(entry.path(), target)?;
        }
    }
    Ok(())
}

/// Remove a path whether it is a file, symlink, or directory.
fn remove_any(path: &Path) -> std::io::Result<()> {
    let meta = match std::fs::symlink_metadata(path) {
        Ok(m) => m,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e),
    };
    if meta.is_dir() && !meta.file_type().is_symlink() {
        std::fs::remove_dir_all(path)
    } else {
        std::fs::remove_file(path)
    }
}

/// Install the neo4j-query skill for detected (or filtered) agents.
pub fn install(agent_filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    // Determine target agents
    let targets: Vec<&Agent> = if let Some(name) = agent_filter {
        let agent = find_agent(name).ok_or_else(|| format!("unknown agent '{name}'"))?;
        vec![agent]
    } else {
        let detected = detect_agents();
        if detected.is_empty() {
            eprintln!("error: no supported AI agents detected. Use --agent to specify one.");
            std::process::exit(1);
        }
        detected
    };

    // Write canonical SKILL.md
    let canonical = canonical_dir().ok_or("cannot determine home directory")?;
    std::fs::create_dir_all(&canonical)?;
    std::fs::write(canonical.join("SKILL.md"), SKILL_MD)?;

    // Install for each target agent
    for agent in &targets {
        let skills_dir = agent
            .skills_path()
            .ok_or_else(|| format!("cannot resolve skills path for {}", agent.display_name))?;
        std::fs::create_dir_all(&skills_dir)?;

        let target_path = skills_dir.join("neo4j-query");
        remove_any(&target_path)?;

        // Try symlink first, fall back to copy
        #[cfg(unix)]
        let link_result = std::os::unix::fs::symlink(&canonical, &target_path);
        #[cfg(not(unix))]
        let link_result: std::io::Result<()> = Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "symlinks not supported",
        ));

        if link_result.is_err() {
            copy_dir(&canonical, &target_path)?;
        }

        println!(
            "installed neo4j-query skill for {} → {}",
            agent.display_name,
            target_path.display()
        );
    }

    Ok(())
}

/// Check whether the neo4j-query skill is installed for a given agent.
fn is_skill_installed(agent: &Agent) -> bool {
    agent
        .skills_path()
        .map(|p| {
            let skill_path = p.join("neo4j-query");
            std::fs::symlink_metadata(&skill_path).is_ok()
        })
        .unwrap_or(false)
}

/// Remove the neo4j-query skill from detected (or filtered) agents.
/// Without --agent, removes from ALL agents that have the skill installed.
pub fn remove(agent_filter: Option<&str>) -> Result<(), Box<dyn std::error::Error>> {
    let targets: Vec<&Agent> = if let Some(name) = agent_filter {
        let agent = find_agent(name).ok_or_else(|| format!("unknown agent '{name}'"))?;
        vec![agent]
    } else {
        AGENTS.iter().filter(|a| is_skill_installed(a)).collect()
    };

    let mut removed_any = false;
    for agent in &targets {
        let skills_dir = match agent.skills_path() {
            Some(p) => p,
            None => continue,
        };
        let target_path = skills_dir.join("neo4j-query");
        if std::fs::symlink_metadata(&target_path).is_ok() {
            remove_any(&target_path)?;
            println!("removed neo4j-query skill from {}", agent.display_name);
            removed_any = true;
        }
    }

    if !removed_any {
        println!("neo4j-query skill was not installed for any agents");
    }

    // If no agent has the skill installed anymore, remove the canonical dir
    let any_installed = AGENTS.iter().any(is_skill_installed);
    if !any_installed {
        if let Some(canonical) = canonical_dir() {
            if std::fs::metadata(&canonical).is_ok() {
                std::fs::remove_dir_all(&canonical)?;
                // Also try to clean up empty parent dirs
                if let Some(parent) = canonical.parent() {
                    let _ = std::fs::remove_dir(parent); // skills/
                    if let Some(grandparent) = parent.parent() {
                        let _ = std::fs::remove_dir(grandparent); // neo4j-query/
                    }
                }
            }
        }
    }

    Ok(())
}

/// List all known agents with their detected and skill-installed status.
pub fn list() {
    let detected_agents = detect_agents();

    println!("{:<15} {:<10} SKILL INSTALLED", "AGENT", "DETECTED");
    for agent in AGENTS {
        let detected = detected_agents.iter().any(|a| a.name == agent.name);
        let installed = is_skill_installed(agent);
        println!(
            "{:<15} {:<10} {}",
            agent.display_name,
            if detected { "yes" } else { "no" },
            if installed { "yes" } else { "no" },
        );
    }
}
