#!/usr/bin/env bash
set -eo pipefail

cd "$(dirname "$0")"
cd fan-control-graphics

# If --watch, run with cargo watch instead
if [ "$1" == "--watch" ]; then
  cargo watch -x "run --example simulate"
  exit
fi
cargo run --example simulate
