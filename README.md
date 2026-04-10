# neo4j-query

A fast, lightweight, non-interactive CLI for querying Neo4j — built for AI agents and humans alike.

## Setup

Two things needed: the **binary** and the **AI agent skill**.

### 1. Install the binary

#### Homebrew (macOS/Linux)

```sh
brew tap oskarhane/neo4j-query
brew install neo4j-query
```

#### From source

```sh
git clone https://github.com/oskarhane/homebrew-neo4j-query.git
cd homebrew-neo4j-query
cargo install --path .
```

### 2. Install the AI agent skill

```sh
neo4j-query skill install
```

This detects supported AI agents (Claude Code, Cursor, Windsurf, Copilot, etc.) and installs the skill for each one. Use `neo4j-query skill list` to see which agents are detected and installed.

Then use `/neo4j-query` in Claude Code to query Neo4j. The skill automatically runs `schema` before generating Cypher so it always uses the correct labels, properties, and relationship types.

### 3. Set credentials

Quickest way — pass directly:

```sh
neo4j-query --uri http://localhost:7474 --username neo4j --password secret "RETURN 1"
```

Or point to an env file:

```sh
neo4j-query --env /path/to/credentials.env "RETURN 1"
```

For repeated use, create a `.env` file in your project directory (auto-discovered):

```sh
echo 'NEO4J_URI=http://localhost:7474
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD=your-password' > .env
```

Shell environment variables (`NEO4J_URI`, `NEO4J_USERNAME`, `NEO4J_PASSWORD`) also work.

## Usage

```sh
# Query as argument
neo4j-query "MATCH (n:Person) RETURN n.name LIMIT 10"

# Query from stdin
echo "MATCH (n) RETURN n LIMIT 5" | neo4j-query

# With parameters
neo4j-query -P name=Alice "MATCH (n:Person {name: \$name}) RETURN n"

# Schema introspection
neo4j-query schema
```

## Configuration

Credentials via `.env` file, environment variables, or CLI flags. Priority: CLI flags > env vars > `.env` file.

| Env var          | Flag         | Default                  |
|------------------|--------------|--------------------------|
| `NEO4J_URI`      | `--uri`      | `http://localhost:7474`  |
| `NEO4J_USERNAME`  | `--username`, `-u` | `neo4j`                  |
| `NEO4J_PASSWORD`  | `--password`, `-p` | *(required)*             |
| `NEO4J_DATABASE`  | `--db`       | `neo4j`                  |
| —                | `--env`      | auto-discover `.env`     |
| —                | `--format`   | `toon`              |

The `--env` flag loads a specific `.env` file. Without it, the tool searches for a `.env` file starting from the current directory and walking up the directory tree.

## Subcommands

| Command              | Description |
|----------------------|-------------|
| `schema`             | Introspect the database schema: node labels, relationship types, properties (with types and mandatory flags), and connection paths |
| `skill install`      | Install the neo4j-query skill for detected AI agents |
| `skill remove`       | Remove the neo4j-query skill from AI agents |
| `skill list`         | List all known AI agents and skill installation status |

## Output

Results are printed to stdout in [TOON format](https://github.com/toon-format/toon-rust), a compact token-efficient serialization format. Errors go to stderr. Use `--format json` for JSON output instead.

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

## Benchmarks

Output defaults to [TOON](https://github.com/toon-format/toon-rust) but JSON is also supported via `--format json`. Token comparison from the recommendations dataset:

| Query | JSON tokens | TOON tokens | Token % |
|-------|------------|------------|--------|
| single_row | 14 | 15 | -7.1% |
| genre_names_5 | 30 | 22 | 26.7% |
| movies_3col_50 | 1,165 | 667 | 42.7% |
| movies_10col_50 | 3,700 | 1,698 | 54.1% |
| movies_4col_500 | 14,300 | 7,765 | 45.7% |
| movies_arrays_50 | 1,413 | 1,855 | -31.3% |
| acted_in_200 | 3,172 | 2,112 | 33.4% |
| ratings_100 | 2,333 | 1,318 | 43.5% |

TOON saves **40-55% tokens** on tabular data. Array-heavy results use TOON's non-tabular encoding which can be larger than JSON.

## License

MIT
