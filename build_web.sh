#!/bin/bash
set -eu
script_path=$( cd "$(dirname "${BASH_SOURCE[0]}")" ; pwd -P )
cd "./"

OPEN=false
FAST=false

while test $# -gt 0; do
  case "$1" in
    -h|--help)
      echo "build_web.sh [--fast] [--open]"
      echo "  --fast: skip optimization step"
      echo "  --open: open the result in a browser"
      exit 0
      ;;
    --fast)
      shift
      FAST=true
      ;;
    --open)
      shift
      OPEN=true
      ;;
    *)
      break
      ;;
  esac
done

# ./setup_web.sh # <- call this first!

CRATE_NAME="gencam_gui"
CRATE_NAME_SNAKE_CASE="${CRATE_NAME//-/_}" # for those who name crates with-kebab-case

# This is required to enable the web_sys clipboard API which egui_web uses
# https://rustwasm.github.io/wasm-bindgen/api/web_sys/struct.Clipboard.html
# https://rustwasm.github.io/docs/wasm-bindgen/web-sys/unstable-apis.html
export RUSTFLAGS=--cfg=web_sys_unstable_apis

# Clear output from old stuff:
rm -f docs/${CRATE_NAME_SNAKE_CASE}_bg.wasm

echo "Building rust…"
BUILD=release
cargo build -p ${CRATE_NAME} --release --lib --target wasm32-unknown-unknown

# Get the output directory (in the workspace it is in another location)
TARGET=`cargo metadata --format-version=1 | jq --raw-output .target_directory`

echo "Generating JS bindings for wasm…"
TARGET_NAME="${CRATE_NAME_SNAKE_CASE}.wasm"
wasm-bindgen "${TARGET}/wasm32-unknown-unknown/${BUILD}/${TARGET_NAME}" \
  --out-dir assets --no-modules --no-typescript

if [ "${FAST}" = false ]; then
  echo "Optimizing wasm…"
  # to get wasm-opt:  apt/brew/dnf install binaryen
  wasm-opt assets/${CRATE_NAME}_bg.wasm -O2 --fast-math -o assets/${CRATE_NAME}_bg.wasm # add -g to get debug symbols
fi

echo "Finished: assets/${CRATE_NAME_SNAKE_CASE}.wasm"

if [ "${OPEN}" = true ]; then
  if [[ "$OSTYPE" == "linux-gnu"* ]]; then
    # Linux, ex: Fedora
    xdg-open http://localhost:8081/index.html
  elif [[ "$OSTYPE" == "msys" ]]; then
    # Windows
    start http://localhost:8081/index.html
  else
    # Darwin/MacOS, or something else
    open http://localhost:8081/index.html
  fi
fi