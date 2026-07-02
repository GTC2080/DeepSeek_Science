#!/usr/bin/env bash
set -euo pipefail

cargo check --workspace
cargo test --workspace --lib
