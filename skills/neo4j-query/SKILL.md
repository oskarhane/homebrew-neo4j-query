---
name: neo4j-query
description: Query Neo4j databases using the neo4j-query CLI tool. Use when the user asks to query Neo4j, explore graph data, run Cypher queries, or fetch data from a Neo4j database.
user-invocable: true
allowed-tools: Bash(neo4j-query *)
argument-hint: "[cypher query]"
---

# Neo4j Query

Query Neo4j databases using `neo4j-query`. Connects via Neo4j's **HTTP API** (not Bolt). Default ports: `http://<host>:7474` or `https://<host>:7473`. When the user says "check my local neo4j" or similar, use `http://localhost:7474` unless they specify otherwise. Output is in TOON format (compact, token-efficient).

## Prerequisites

`neo4j-query` must be installed and in PATH. Credentials can be set via CLI flags, a `.env` file, or environment variables:

| Flag | Env var | Default |
|------|---------|---------|
| `--uri` | `NEO4J_URI` | `http://localhost:7474` |
| `-u` / `--username` | `NEO4J_USERNAME` | `neo4j` |
| `-p` / `--password` | `NEO4J_PASSWORD` | *(required)* |
| `--db` | `NEO4J_DATABASE` | `neo4j` |
| `--env` | — | auto-discover `.env` |

Priority: CLI flags > env vars > `.env` file. Use `--env path` to load a specific env file. Prefer `-u`/`-p` over `--username`/`--password` when passing credentials on the command line.

## CRITICAL: Fetch schema before generating Cypher

**Before generating ANY Cypher yourself, ALWAYS run `schema` first** to understand the database structure. Do not guess label names, relationship types, or property names — get them from the schema. If the user provides a Cypher query directly, just execute it — no need to fetch schema first.

```bash
neo4j-query schema
```

The `schema` subcommand returns a structured TOON object with:
- **nodes**: every node label with its properties (name, type, mandatory flag)
- **relationships**: every relationship type with its properties AND `paths` showing which node labels it connects (from → to)

Example output:
```
nodes[2]:
  - labels[1]: Person
    properties[2]:
      - name: name
        types[1]: String
        mandatory: true
      - name: age
        types[1]: Long
        mandatory: false
  - labels[1]: Company
    properties[1]:
      - name: name
        types[1]: String
        mandatory: true
relationships[1]:
  - type: WORKS_AT
    properties[1]:
      - name: since
        types[1]: Long
        mandatory: true
    paths[1]:
      - from[1]: Person
        to[1]: Company
```

Use this output to:
1. Know exactly which labels exist (don't guess `User` when it's `Person`)
2. Know property names and types (don't guess `username` when it's `name`)
3. Know relationship types and directions (don't guess `EMPLOYED_BY` when it's `WORKS_AT`, and know it goes Person→Company not the reverse)
4. Write correct, targeted Cypher queries on the first try

## How to use

Run Cypher queries with:
```bash
neo4j-query "MATCH (n:Person) RETURN n.name, n.age LIMIT 10"
```

With parameters:
```bash
neo4j-query -P name=Alice -P age=30 "MATCH (n:Person {name: \$name, age: \$age}) RETURN n"
```

Query from stdin:
```bash
echo "MATCH (n) RETURN labels(n), count(*)" | neo4j-query
```

## When given a user argument

If the user provides a Cypher query as $ARGUMENTS, run it directly:
```bash
neo4j-query "$ARGUMENTS"
```

If the user asks a question about the data or asks you to explore/query **without providing Cypher**:
1. Run `neo4j-query schema` first
2. Use the schema to write the correct Cypher query
3. Run the query

If the user provides a specific Cypher query, run it directly — don't fetch schema first.

## Tips

- Run `neo4j-query schema` before generating Cypher — never assume you know the schema
- Use `LIMIT` for exploratory queries to avoid large result sets
- Use parameters (`-P`) for dynamic values instead of string interpolation
- Relationship directions matter — check `paths.from` and `paths.to` in the schema output
- Property types from the schema tell you whether to use string vs numeric comparisons
- Use `--format json` if you need JSON output instead of TOON
- Use `--truncate-arrays-over N` to replace arrays longer than N items (default 100, 0 disables). Useful for hiding embedding vectors. TOON shows `[array truncated: N items]`, JSON uses `[]`
