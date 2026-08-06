#!/usr/bin/env bash
set -euo pipefail

project_dir="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

"$project_dir/build.sh"
exec "$project_dir/dist/TermXBoard" "$@"
