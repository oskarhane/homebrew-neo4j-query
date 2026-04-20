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

# Truncate large arrays (e.g. embedding vectors)
# Default: arrays > 100 items are replaced. Use 0 to disable.
neo4j-query --truncate-arrays-over 50 "MATCH (n) RETURN n LIMIT 5"
```

## Embeddings

Generate embedding vectors inline for Neo4j vector search. Opt-in: set `NEO4J_EMBED_PROVIDER` to either `openai` or `ollama`. With no provider configured, the CLI behaves exactly as before and pays no embed cost.

### `:embed` param modifier

Add `:embed` to a parameter name in query mode and the CLI replaces the text with a `Vec<f32>` before sending the query:

```sh
neo4j-query -P q:embed='science fiction movies about AI' \
  "CALL db.index.vector.queryNodes('movie_embeddings', 5, \$q)
   YIELD node, score
   RETURN node.title AS title, score"
```

Other `-P` params keep their normal type coercion:

```sh
neo4j-query -P k=5 -P q:embed='sci-fi movies' \
  "CALL db.index.vector.queryNodes('movie_embeddings', \$k, \$q)
   YIELD node, score RETURN node.title, score"
```

### `embed` subcommand

Debug / scripting helper. Prints the embedding for stdin or a positional argument.

```sh
# JSON array (default)
neo4j-query embed 'hello world'

# Newline-separated floats (useful with wc -l, paste, etc.)
neo4j-query embed --format raw 'hello world'

# Stdin
echo 'hello world' | neo4j-query embed
```

### Setup: Ollama (local, free)

Run Ollama locally, pull a model, point the CLI at it:

```sh
ollama serve &
ollama pull all-minilm

cat > .env <<'EOF'
NEO4J_URI=http://localhost:7474
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD=your-password
NEO4J_EMBED_PROVIDER=ollama
NEO4J_EMBED_MODEL=all-minilm
# NEO4J_EMBED_BASE_URL=http://localhost:11434   # default
EOF
```

No API key needed. Ollama is unreachable → error names `ollama serve`; HTTP 404 → error names `ollama pull <model>`.

### Setup: OpenAI (hosted)

```sh
cat > .env <<'EOF'
NEO4J_URI=http://localhost:7474
NEO4J_USERNAME=neo4j
NEO4J_PASSWORD=your-password
NEO4J_EMBED_PROVIDER=openai
NEO4J_EMBED_MODEL=text-embedding-3-small
# NEO4J_EMBED_DIMENSIONS=1536     # optional, OpenAI only
OPENAI_API_KEY=sk-...
EOF
```

`OPENAI_API_KEY` is preferred; `NEO4J_EMBED_API_KEY` is used as a fallback.

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
| —                | `--truncate-arrays-over` | `100` (0 disables) |
| `NEO4J_EMBED_PROVIDER` | `--embed-provider` | *(unset — opt-in)* |
| `NEO4J_EMBED_MODEL` | `--embed-model` | *(required when provider set)* |
| `NEO4J_EMBED_DIMENSIONS` | `--embed-dimensions` | *(optional, OpenAI only)* |
| `NEO4J_EMBED_BASE_URL` | `--embed-base-url` | provider default |
| `OPENAI_API_KEY` | — | *(required for `openai`)* |
| `NEO4J_EMBED_API_KEY` | — | fallback for `OPENAI_API_KEY` |

The `--env` flag loads a specific `.env` file. Without it, the tool searches for a `.env` file starting from the current directory and walking up the directory tree.

Embedding configuration is opt-in. Unset `NEO4J_EMBED_PROVIDER` means the CLI never touches embed env vars or builds an HTTP client for embeddings.

## Subcommands

| Command              | Description |
|----------------------|-------------|
| `schema`             | Introspect the database schema: node labels, relationship types, properties (with types and mandatory flags), and connection paths |
| `embed [TEXT]`       | Embed text via the configured provider; reads stdin when `TEXT` is omitted. Use `--format json` (default) or `--format raw` (one float per line) |
| `skill install [--agent <name>]` | Install the neo4j-query skill for detected AI agents (or a specific one) |
| `skill remove [--agent <name>]`  | Remove the neo4j-query skill from AI agents (or a specific one) |
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
