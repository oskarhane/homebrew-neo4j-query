# PRD: Skill Management & Schema Subcommands

## Overview

Add subcommands to `neo4j-query` for managing AI agent skill installation and for schema introspection. The `skill` command detects installed AI coding agents, writes a canonical copy of the bundled SKILL.md, and symlinks it into each agent's global skills directory. The `schema` command replaces the old `.schema` magic string with a proper subcommand.

## Goals

- Let users install the neo4j-query skill to all detected AI agents with a single command
- Use symlinks from a single canonical source so updates only need to happen once
- Support 11 agents: claude-code, cursor, windsurf, copilot, gemini-cli, cline, roo, codex, pi, opencode, junie
- Make `schema` a proper subcommand (drop `.schema` dot prefix, no backward compat)

## Non-Goals

- Project-level (non-global) skill installation
- Supporting agents not in the initial 11
- Custom/user-authored skill content
- Backward compatibility for `.schema`

## Requirements

### Functional Requirements

- REQ-F-001: Add `skill install [--agent <name>]` subcommand. Detects all agents by default, or targets a specific agent via `--agent`. Writes canonical SKILL.md to `~/.local/share/neo4j-query/skills/neo4j-query/`, then symlinks `<agent_skills_dir>/neo4j-query` → canonical dir. If symlink fails, falls back to copying the directory.
- REQ-F-002: Add `skill remove [--agent <name>]` subcommand. Removes symlinks (or copies) from agent skill dirs. If no agents remain, removes canonical dir too.
- REQ-F-003: Add `skill list` subcommand. For each of the 11 known agents, prints whether detected (config dir exists) and whether the skill is installed (symlink/dir exists in agent's skills dir).
- REQ-F-004: If skill already exists at target (file, dir, or symlink), remove and recreate on install (overwrite).
- REQ-F-005: If no agents detected and no `--agent` flag, print `no agents detected. use --agent to specify one.` to stderr and exit 1.
- REQ-F-006: Add `schema` subcommand with identical behavior to old `.schema` — introspects database schema, outputs TOON. Requires same connection flags (`--uri`, `-u`, `-p`, `--db`, `--env`).
- REQ-F-007: Remove `.schema` magic string handling. `neo4j-query .schema` should fail as an unrecognized input.
- REQ-F-008: Positional query argument (`neo4j-query "MATCH ..."`) continues to work unchanged when no subcommand is given.
- REQ-F-009: Skill content is embedded in the binary at compile time via `include_str!` from `skills/neo4j-query/SKILL.md`.
- REQ-F-010: Agent detection checks these global config directories via `stat()`:

  | Agent | Detect dir | Skills dir |
  |-------|-----------|-----------|
  | claude-code | `~/.claude` | `~/.claude/skills` |
  | cursor | `~/.cursor` | `~/.cursor/skills` |
  | windsurf | `~/.codeium/windsurf` | `~/.codeium/windsurf/skills` |
  | copilot | `~/.copilot` | `~/.copilot/skills` |
  | gemini-cli | `~/.gemini` | `~/.gemini/skills` |
  | cline | `~/.cline` | `~/.agents/skills` |
  | roo | `~/.roo` | `~/.roo/skills` |
  | codex | `~/.codex` | `~/.codex/skills` |
  | pi | `~/.pi/agent` | `~/.pi/agent/skills` |
  | opencode | `$XDG_CONFIG_HOME/opencode` | `$XDG_CONFIG_HOME/opencode/skills` |
  | junie | `~/.junie` | `~/.junie/skills` |

### Non-Functional Requirements

- REQ-NF-001: No new Cargo dependencies. Use `std::os::unix::fs::symlink` (with `std::fs::copy` fallback) and `std::fs` for all file operations.
- REQ-NF-002: Skill logic lives in a new `src/skill.rs` module, not inlined into `main.rs`.
- REQ-NF-003: Existing unit and integration tests continue to pass. Integration tests referencing `.schema` must be updated to use `schema`.
- REQ-NF-004: Update `README.md`: replace skill install instructions with `neo4j-query skill install`, update `.schema` → `schema` everywhere, update Built-in Commands table to list all new subcommands.
- REQ-NF-005: Update `.claude/skills/neo4j-query/SKILL.md`: replace `.schema` → `schema` in all references and examples.
- REQ-NF-006: Create `skills/neo4j-query/SKILL.md` (compile-time embed source) as a copy of the updated `.claude/skills/neo4j-query/SKILL.md`.
- REQ-NF-007: `$XDG_CONFIG_HOME` defaults to `~/.config` when unset (for opencode agent detection).

## Technical Considerations

- **Clap subcommand + positional arg coexistence**: Use `#[command(args_conflicts_with_subcommands = true)]` on the `Cli` struct with an `Option<Commands>` subcommand field. Extract existing query/connection args into a flattened `QueryArgs` struct. When subcommand is `None`, run query mode. When `Some(Commands::Schema)`, run schema mode with the same connection args.
- **Schema subcommand needs connection flags**: The `Schema` variant must accept the same connection flags (`--uri`, `-u`, `-p`, `--db`, `--env`) as query mode. Flatten `ConnectionArgs` into both `QueryArgs` and `Schema`.
- **Symlink target**: The symlink points at the `~/.local/share/neo4j-query/skills/neo4j-query/` directory (not individual files), so the entire skill directory is linked.
- **Overwrite safety**: On install, if the target path exists (whether symlink, file, or directory), `remove_all` before recreating. This handles upgrading from copy → symlink or vice versa.

## Acceptance Criteria

- [ ] `neo4j-query skill install` detects agents and creates symlinks; prints one line per agent
- [ ] `neo4j-query skill install --agent claude-code` installs only for Claude Code
- [ ] `neo4j-query skill install` on second run overwrites cleanly (no error)
- [ ] `ls -la ~/.claude/skills/neo4j-query` shows symlink → `~/.local/share/neo4j-query/skills/neo4j-query/`
- [ ] `neo4j-query skill remove` removes symlinks and canonical dir
- [ ] `neo4j-query skill remove --agent claude-code` removes only Claude Code's symlink
- [ ] `neo4j-query skill list` shows detected/installed status per agent
- [ ] `neo4j-query skill install` with no agents detected prints error to stderr, exits 1
- [ ] `neo4j-query schema` outputs TOON schema (same as old `.schema`)
- [ ] `neo4j-query .schema` fails (unrecognized)
- [ ] `neo4j-query "RETURN 1"` still works (query mode unchanged)
- [ ] `cargo build` compiles
- [ ] `cargo test` passes
- [ ] `cargo test -- --ignored` passes (integration tests updated)
- [ ] README.md updated: skill install instructions, `.schema` → `schema`, commands table
- [ ] `.claude/skills/neo4j-query/SKILL.md` updated: `.schema` → `schema`
- [ ] `skills/neo4j-query/SKILL.md` exists as embed source with updated content

## Out of Scope

- Project-level / `--project` flag for skill installation
- `skill update` command (install already overwrites)
- Version tracking in SKILL.md frontmatter
- Detecting agents by environment variables or runtime context
- Adding new agents beyond the initial 11

## Open Questions

None.
