# neo4j-query

Query Neo4j databases from the command line. Outputs results in [TOON](https://github.com/toon-format/toon-rust) format.

## Setup

Two things needed: the **binary** and the **Claude Code skill**.

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

### 2. Install the Claude Code skill

```sh
npx skills add https://github.com/oskarhane/homebrew-neo4j-query
```

Or manually:

```sh
cp -r .claude/skills/neo4j-query ~/.claude/skills/
```

Then use `/neo4j-query` in Claude Code to query Neo4j. The skill automatically runs `.schema` before generating Cypher so it always uses the correct labels, properties, and relationship types.

### 3. Set credentials

Via a `.env` file (recommended):

```sh
# Create a .env file in your project directory
echo 'NEO4J_URI=http://localhost:7474
NEO4J_USER=neo4j
NEO4J_PASSWORD=your-password' > .env
```

The tool automatically discovers `.env` files by searching from the current directory upward. You can also specify one explicitly:

```sh
neo4j-query --env /path/to/credentials.env "RETURN 1"
```

Or via shell environment variables:

```sh
export NEO4J_URI="http://localhost:7474"
export NEO4J_USER="neo4j"
export NEO4J_PASSWORD="your-password"
```

## Usage

```sh
# Query as argument
neo4j-query "MATCH (n:Person) RETURN n.name LIMIT 10"

# Query from stdin
echo "MATCH (n) RETURN n LIMIT 5" | neo4j-query

# With parameters
neo4j-query -p name=Alice "MATCH (n:Person {name: \$name}) RETURN n"

# Schema introspection
neo4j-query .schema
```

## Configuration

Credentials via `.env` file, environment variables, or CLI flags. Priority: CLI flags > env vars > `.env` file.

| Env var          | Flag         | Default                  |
|------------------|--------------|--------------------------|
| `NEO4J_URI`      | `--uri`      | `http://localhost:7474`  |
| `NEO4J_USER`     | `--user`     | `neo4j`                  |
| `NEO4J_PASSWORD`  | `--password` | *(required)*             |
| `NEO4J_DATABASE`  | `--database` | `neo4j`                  |
| —                | `--env`      | auto-discover `.env`     |

The `--env` flag loads a specific `.env` file. Without it, the tool searches for a `.env` file starting from the current directory and walking up the directory tree.

## Built-in Commands

| Command    | Description |
|------------|-------------|
| `.schema`  | Introspect the database schema: node labels, relationship types, properties (with types and mandatory flags), and connection paths |

## Output

Results are printed to stdout in [TOON format](https://github.com/toon-format/toon-rust), a compact token-efficient serialization format. Errors go to stderr.

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
