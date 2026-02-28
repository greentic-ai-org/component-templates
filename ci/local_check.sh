#!/usr/bin/env bash
set -euo pipefail

cargo fmt --all
tools/i18n.sh validate
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace --all-targets
