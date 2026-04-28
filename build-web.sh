#!/usr/bin/env bash
# Build rust-stones for the web and produce an itch.io-ready zip.
#
# Prereqs (one-time):
#   rustup target add wasm32-unknown-unknown
#   cargo install wasm-bindgen-cli --version 0.2.118
#   (optional) install binaryen so `wasm-opt` is on PATH for size-optimised wasm
#
# Output: web/ (servable as-is) and rust-stones-web.zip (upload to itch).

set -euo pipefail

echo "==> cargo build --release --target wasm32-unknown-unknown"
cargo build --release --target wasm32-unknown-unknown

echo "==> wasm-bindgen"
wasm-bindgen \
  --no-typescript --target web \
  --out-dir ./web/ --out-name "rust-stones" \
  ./target/wasm32-unknown-unknown/release/rust-stones.wasm

echo "==> copy assets"
rm -rf ./web/assets
cp -r ./assets ./web/assets

# wasm-opt is optional but trims ~30-50% off the binary. Skip silently
# if binaryen isn't installed so the script still works on a fresh box.
if command -v wasm-opt >/dev/null 2>&1; then
  echo "==> wasm-opt -Oz"
  wasm-opt -Oz -o ./web/rust-stones_bg.wasm ./web/rust-stones_bg.wasm
else
  echo "(skip) wasm-opt not on PATH — install binaryen for a smaller wasm binary"
fi

echo "==> packaging rust-stones-web.zip"
rm -f rust-stones-web.zip
# Use PowerShell's Compress-Archive on Windows since `zip` may not be
# installed; on macOS / Linux the `zip` utility is the natural choice.
if command -v zip >/dev/null 2>&1; then
  (cd web && zip -r ../rust-stones-web.zip .)
else
  powershell -NoProfile -Command "Compress-Archive -Path ./web/* -DestinationPath rust-stones-web.zip"
fi

wasm_bytes=$(stat -c %s ./web/rust-stones_bg.wasm 2>/dev/null || stat -f %z ./web/rust-stones_bg.wasm)
zip_bytes=$(stat -c %s ./rust-stones-web.zip 2>/dev/null || stat -f %z ./rust-stones-web.zip)
printf "==> done. wasm: %.2f MB, zip: %.2f MB\n" \
  "$(awk "BEGIN { print $wasm_bytes / 1048576 }")" \
  "$(awk "BEGIN { print $zip_bytes / 1048576 }")"
echo "    Upload rust-stones-web.zip to itch.io as an HTML game."
