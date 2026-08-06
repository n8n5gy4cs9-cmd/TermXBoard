#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

if ! command -v cargo >/dev/null 2>&1; then
  echo "Error: Rust/Cargo is required. Install it from https://rustup.rs/" >&2
  exit 1
fi

"$project_dir/build.sh"
exec "$project_dir/dist/TermXBoard" "$@"
