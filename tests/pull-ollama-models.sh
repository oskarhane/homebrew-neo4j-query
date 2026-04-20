#!/usr/bin/env bash
# Pull embedding models into the local Ollama service started by
# tests/docker-compose.yml. Run after `docker compose -f tests/docker-compose.yml up -d`.
set -euo pipefail

CONTAINER="${OLLAMA_CONTAINER:-tests-ollama-1}"

docker exec "$CONTAINER" ollama pull all-minilm
