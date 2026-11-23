#!/usr/bin/env bash
# Build the browser version into a folder (default: ../webroot):
#   scripts/build-web.sh [out-dir] [--release]
# Needs the wasm32 target and wasm-bindgen-cli matching Cargo.lock's version
# (cargo binstall wasm-bindgen-cli@<version>). See docs/release-and-updates-plan.md.
set -euo pipefail
cd "$(dirname "$0")/.."
out="${1:-../webroot}"
profile=debug; flag=""
if [ "${2:-}" = "--release" ]; then profile=release; flag="--release"; fi
cargo build --target wasm32-unknown-unknown $flag --bin portmanteau
mkdir -p "$out"
wasm-bindgen --target web --no-typescript --out-dir "$out" --out-name portmanteau \
  "target/wasm32-unknown-unknown/$profile/portmanteau.wasm"
cp web/index.html "$out/index.html"
cp assets/icon/icon.png "$out/icon.png"
ls -l "$out"
