#!/bin/bash
set -e

echo "Building WASM..."
cargo build --target wasm32-unknown-unknown --bin editor

echo "Copying to root..."
cp target/wasm32-unknown-unknown/debug/editor.wasm .

echo ""
echo "Ready! Starting server on http://localhost:8080"
echo "Press Ctrl+C to stop"
echo ""

python3 -m http.server 8080
