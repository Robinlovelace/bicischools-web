#!/usr/bin/env bash
set -euo pipefail

echo "Building bicischools WebAssembly bindings..."
cd "$(dirname "$0")/../bindings/wasm"
wasm-pack build --release --target bundler --out-dir ../../web/src/lib/wasm

echo "WASM build complete! Output in web/src/lib/wasm"
