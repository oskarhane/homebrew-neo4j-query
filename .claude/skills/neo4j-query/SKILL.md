---
name: neo4j-query
description: Query Neo4j databases using the neo4j-query CLI tool. Use when the user asks to query Neo4j, explore graph data, run Cypher queries, or fetch data from a Neo4j database.
user-invocable: true
allowed-tools: Bash(neo4j-query *)
argument-hint: "[cypher query]"
---

# Neo4j Query

Query Neo4j databases using `neo4j-query`. Output is in TOON format (compact, token-efficient).

## Prerequisites

`neo4j-query` must be installed and in PATH. Credentials must be set via environment variables:
- `NEO4J_URI` — Neo4j HTTP endpoint (default: `http://localhost:7474`)
- `NEO4J_USER` — username (default: `neo4j`)
- `NEO4J_PASSWORD` — password (required)
- `NEO4J_DATABASE` — database name (default: `neo4j`)

## How to use

Run Cypher queries with:
```bash
neo4j-query "MATCH (n:Person) RETURN n.name, n.age LIMIT 10"
```

With parameters:
```bash
neo4j-query -p name=Alice -p age=30 "MATCH (n:Person {name: \$name, age: \$age}) RETURN n"
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

## Tips

- Always use `LIMIT` for exploratory queries to avoid large result sets
- Use parameters (`-p`) for dynamic values instead of string interpolation
- Start with schema exploration: `neo4j-query "CALL db.schema.visualization()"`
- List labels: `neo4j-query "CALL db.labels()"`
- List relationship types: `neo4j-query "CALL db.relationshipTypes()"`
