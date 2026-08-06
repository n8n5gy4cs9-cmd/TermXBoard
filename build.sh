#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: Rust/Cargo is required. Install it from https://rustup.rs/" >&2
  exit 1
fi

cd "$project_dir"
cargo build --release
mkdir -p "$project_dir/dist"
cp "$project_dir/target/release/TermXBoard" "$project_dir/dist/TermXBoard"
chmod +x "$project_dir/dist/TermXBoard"
echo "Built $project_dir/dist/TermXBoard"
