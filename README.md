# neo4j-query

Query Neo4j databases from the command line. Outputs results in [TOON](https://github.com/toon-format/toon-rust) format.

## Install

### Homebrew (macOS/Linux)

```sh
brew tap oskarhane/neo4j-query
brew install neo4j-query
```

### From source

```sh
cargo install --path .
```

## Usage

```sh
# Query as argument
neo4j-query "MATCH (n:Person) RETURN n.name LIMIT 10"

# Query from stdin
echo "MATCH (n) RETURN n LIMIT 5" | neo4j-query

# With parameters
neo4j-query -p name=Alice "MATCH (n:Person {name: \$name}) RETURN n"
```

## Configuration

Credentials via environment variables or CLI flags. CLI flags take priority.

| Env var          | Flag         | Default                  |
|------------------|--------------|--------------------------|
| `NEO4J_URI`      | `--uri`      | `http://localhost:7474`  |
| `NEO4J_USER`     | `--user`     | `neo4j`                  |
| `NEO4J_PASSWORD`  | `--password` | *(required)*             |
| `NEO4J_DATABASE`  | `--database` | `neo4j`                  |

```sh
# Set once in your shell profile
export NEO4J_URI="http://localhost:7474"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="your-password"
```

## Output

Results are printed to stdout in [TOON format](https://github.com/toon-format/toon-rust), a compact token-efficient serialization format. Errors go to stderr.

## Claude Code Skill

This repo includes a Claude Code skill at `.claude/skills/neo4j-query/`. To use it, copy the skill to your project or personal skills directory:

```sh
cp -r .claude/skills/neo4j-query ~/.claude/skills/
```

Then use `/neo4j-query` in Claude Code to query Neo4j databases.

## Development

```sh
# Build
cargo build

# Unit tests
cargo test

# Integration tests (requires Docker)
docker compose -f tests/docker-compose.yml up -d
cargo test -- --ignored
docker compose -f tests/docker-compose.yml down
```

## License

MIT
