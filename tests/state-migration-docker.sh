#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Build only on host. Every executable test gets a disposable container HOME.
cargo build --locked
cargo test --locked --no-run --message-format=json > target/state-test-artifacts.jsonl
runtime=envx-state-test:local
docker build -t "$runtime" -f tests/state-runtime.Dockerfile tests
binary=$(python3 -c 'import json; print(next(x["executable"] for x in map(json.loads,open("target/state-test-artifacts.jsonl")) if x.get("reason")=="compiler-artifact" and x.get("profile",{}).get("test") and x.get("executable")))')
docker run --rm --network none -v "$binary:/artifacts/unit-tests:ro" "$runtime" /artifacts/unit-tests --test-threads=8
docker run --rm --network none -v "$PWD/target/debug/envx:/artifacts/envx:ro" -v "$PWD/tests/fixtures/state:/fixtures:ro" -v "$PWD/tests/state_migration.py:/tests.py:ro" "$runtime" python3 /tests.py
