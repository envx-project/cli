#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")/.."
# Compile on host without executing tests; Cargo's real parallel test command
# subsequently runs inside Docker, with no host HOME or credential mounts.
cargo test --locked --no-run
rust_sysroot=$(rustc --print sysroot)
cargo_registry_root="${CARGO_HOME:-$HOME/.cargo}"
runtime=envx-home-isolation-test:local
docker build -t "$runtime" -f tests/test-home-isolation.Dockerfile tests
docker run --rm --network none \
  -e "PATH=$rust_sysroot/bin:/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin" \
  -e "CARGO_HOME=$cargo_registry_root" \
  -v "$rust_sysroot:$rust_sysroot:ro" \
  -v "$cargo_registry_root/registry:$cargo_registry_root/registry:ro" \
  -v "$PWD:$PWD:ro" -v "$PWD/target:$PWD/target" \
  -w "$PWD" "$runtime" python3 tests/test-home-isolation.py
