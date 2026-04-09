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

## CRITICAL: Always fetch schema first

**Before writing ANY Cypher query, ALWAYS run `.schema` first** to understand the database structure. Do not guess label names, relationship types, or property names — get them from the schema.

```bash
neo4j-query .schema
```

The `.schema` command returns a structured TOON object with:
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

If the user asks a question about the data or asks you to explore/query without providing Cypher:
1. Run `neo4j-query .schema` first
2. Use the schema to write the correct Cypher query
3. Run the query

## Tips

- Always run `neo4j-query .schema` first — never assume you know the schema
- Use `LIMIT` for exploratory queries to avoid large result sets
- Use parameters (`-p`) for dynamic values instead of string interpolation
- Relationship directions matter — check `paths.from` and `paths.to` in the schema output
- Property types from the schema tell you whether to use string vs numeric comparisons
