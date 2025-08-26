#!/usr/bin/env bash
set -euo pipefail
cd "$(dirname "$0")"
cargo build --release --target wasm32-unknown-unknown
OUT_DIR="../../target/wasm32-unknown-unknown/release"
OUT1="$OUT_DIR/echo-wasm.wasm"
OUT2="$OUT_DIR/echo_wasm.wasm"
if [ -f "$OUT1" ]; then
	echo "Built: $OUT1"
elif [ -f "$OUT2" ]; then
	echo "Built: $OUT2"
else
	echo "Build succeeded but .wasm not found at $OUT1 or $OUT2"
	ls -lah "$OUT_DIR" || true
	exit 1
fi
