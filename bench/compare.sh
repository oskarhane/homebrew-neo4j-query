#!/usr/bin/env bash
set -euo pipefail

# TOON vs JSON benchmark for neo4j-query
# Usage: ./bench/compare.sh --env ~/recommendations.env

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"
PROJECT_DIR="$(cd "$SCRIPT_DIR/.." && pwd)"
RUNS=5

# Pass all args through to neo4j-query (e.g. --env ~/recommendations.env)
NEO4J_ARGS=("$@")

# Queries: name|cypher
QUERIES=(
  "single_row|RETURN 1 AS n, 'hello' AS s, true AS b"
  "genre_names_5|MATCH (g:Genre) RETURN g.name LIMIT 5"
  "movies_3col_50|MATCH (m:Movie) RETURN m.title, m.year, m.imdbRating LIMIT 50"
  "movies_10col_50|MATCH (m:Movie) RETURN m.title, m.year, m.imdbRating, m.runtime, m.budget, m.revenue, m.imdbVotes, m.imdbId, m.movieId, m.tmdbId LIMIT 50"
  "movies_4col_500|MATCH (m:Movie) RETURN m.title, m.year, m.imdbRating, m.runtime LIMIT 500"
  "movies_arrays_50|MATCH (m:Movie) RETURN m.title, m.year, m.countries, m.languages LIMIT 50"
  "acted_in_200|MATCH (m:Movie)<-[:ACTED_IN]-(a:Actor) RETURN m.title, a.name LIMIT 200"
  "ratings_100|MATCH (u:User)-[r:RATED]->(m:Movie) RETURN u.name, m.title, r.rating LIMIT 100"
)

NQ="$PROJECT_DIR/target/release/neo4j-query"
TC="$PROJECT_DIR/target/release/bench-token-count"

echo "Building release binaries..."
cargo build --release --features bench --manifest-path "$PROJECT_DIR/Cargo.toml" 2>&1 | tail -1

if [[ ! -x "$NQ" ]] || [[ ! -x "$TC" ]]; then
  echo "error: build failed" >&2
  exit 1
fi

TMPDIR_BENCH=$(mktemp -d)
trap 'rm -rf "$TMPDIR_BENCH"' EXIT

# Run a query and capture output to file, return elapsed time in ms
run_timed() {
  local outfile="$1"
  local format="$2"
  local cypher="$3"
  local start end
  start=$(perl -MTime::HiRes=time -e 'printf "%.0f\n", time*1000')
  "$NQ" "${NEO4J_ARGS[@]}" --output "$format" "$cypher" > "$outfile" 2>/dev/null
  end=$(perl -MTime::HiRes=time -e 'printf "%.0f\n", time*1000')
  echo $(( end - start ))
}

# Header
printf "\n## TOON vs JSON Benchmark Results\n\n"
printf "| %-20s | %11s | %11s | %8s | %9s | %9s |\n" \
  "Query" "JSON tokens" "TOON tokens" "Token %" "JSON ms" "TOON ms"
printf "|%s|%s|%s|%s|%s|%s|\n" \
  "$(printf -- '-%.0s' {1..22})" \
  "$(printf -- '-%.0s' {1..13})" \
  "$(printf -- '-%.0s' {1..13})" \
  "$(printf -- '-%.0s' {1..10})" \
  "$(printf -- '-%.0s' {1..11})" \
  "$(printf -- '-%.0s' {1..11})"

for entry in "${QUERIES[@]}"; do
  name="${entry%%|*}"
  cypher="${entry#*|}"

  json_file="$TMPDIR_BENCH/${name}_json.out"
  toon_file="$TMPDIR_BENCH/${name}_toon.out"

  # Size/token measurement (single run, capture output)
  "$NQ" "${NEO4J_ARGS[@]}" --output json "$cypher" > "$json_file" 2>/dev/null
  "$NQ" "${NEO4J_ARGS[@]}" --output toon "$cypher" > "$toon_file" 2>/dev/null

  read _ json_tokens <<< "$("$TC" < "$json_file")"
  read _ toon_tokens <<< "$("$TC" < "$toon_file")"

  if (( json_tokens > 0 )); then
    token_pct=$(awk "BEGIN { printf \"%.1f\", (1 - $toon_tokens/$json_tokens) * 100 }")
  else
    token_pct="0.0"
  fi

  # Speed measurement (multiple runs, average)
  json_total=0
  toon_total=0
  for (( i=0; i<RUNS; i++ )); do
    t=$(run_timed /dev/null json "$cypher")
    json_total=$(( json_total + t ))
    t=$(run_timed /dev/null toon "$cypher")
    toon_total=$(( toon_total + t ))
  done
  json_avg=$(( json_total / RUNS ))
  toon_avg=$(( toon_total / RUNS ))

  printf "| %-20s | %11d | %11d | %7s%% | %7d ms | %7d ms |\n" \
    "$name" "$json_tokens" "$toon_tokens" "$token_pct" "$json_avg" "$toon_avg"
done

printf "\n*%d runs averaged for timing. Token counts use cl100k_base tokenizer.*\n" "$RUNS"
