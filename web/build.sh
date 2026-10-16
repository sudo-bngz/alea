#!/usr/bin/env bash
# Builds alea for the browser: web/dist/alea.wasm, then the playground page.
# Needs the wasm32 standard library: `rustup target add wasm32-unknown-unknown`.
# (Without it, and with the Rust sources installed, this falls back to building
# the standard library itself via -Zbuild-std.)
set -euo pipefail
cd "$(dirname "$0")/.."
mkdir -p web/dist
if rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
  cargo build --release --lib --target wasm32-unknown-unknown --no-default-features
else
  RUSTC_BOOTSTRAP=1 cargo build --release --lib --target wasm32-unknown-unknown --no-default-features \
    -Zbuild-std=std,panic_abort
fi
cp target/wasm32-unknown-unknown/release/alea.wasm web/dist/alea.wasm
ls -la web/dist/alea.wasm
python3 "$(dirname "$0")/bundle.py"
