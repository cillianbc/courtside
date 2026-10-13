#!/usr/bin/env bash
# Builds courtside-ffi for WebAssembly and generates the JS glue:
#   target/wasm/web     ES module for browsers (the bench viewer, phase 4)
#   target/wasm/nodejs  CommonJS for Node (smoke test)
# Needs the wasm32-unknown-unknown target and wasm-bindgen-cli at the exact
# version pinned in the root Cargo.toml:
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version <pinned> --locked
set -euo pipefail
root="$(cd "$(dirname "$0")/../.." && pwd)"
cd "$root"
cargo build --release --target wasm32-unknown-unknown -p courtside-ffi
wasm="target/wasm32-unknown-unknown/release/courtside_ffi.wasm"
for target in web nodejs; do
  rm -rf "target/wasm/$target"
  wasm-bindgen --target "$target" --out-dir "target/wasm/$target" "$wasm"
done
ls -l target/wasm/web/*.wasm
