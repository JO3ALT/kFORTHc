#!/usr/bin/env bash
set -euo pipefail

KPASCAL_BIN="${KPASCAL_BIN:-../kpascal/target/release/kpascal}"
SAMPLES_DIR="samples"
BUILD_DIR="samples/build"
mkdir -p "$BUILD_DIR"

if [[ ! -x "$KPASCAL_BIN" ]]; then
  echo "error: kpascal binary not found: $KPASCAL_BIN" >&2
  exit 1
fi

if command -v llc >/dev/null 2>&1; then
  LLC=llc
elif command -v llc-14 >/dev/null 2>&1; then
  LLC=llc-14
else
  echo "error: llc not found (tried: llc, llc-14)" >&2
  exit 1
fi

cargo build >/dev/null

name="16_divmod_zero_runtime"
src="$SAMPLES_DIR/$name.pas"
forth="$BUILD_DIR/$name.fth"
ir="$BUILD_DIR/$name.ll"
obj="$BUILD_DIR/$name.o"
bin="$BUILD_DIR/$name.out"

"$KPASCAL_BIN" < "$src" > "$forth"
./target/debug/kforthc "$forth" "$ir"
"$LLC" -filetype=obj "$ir" -o "$obj"
clang -no-pie "$obj" runtime/runtime.c -o "$bin" -lm

set +e
"$bin" > "$BUILD_DIR/$name.actual" 2> "$BUILD_DIR/$name.err"
rc=$?
set -e

if [[ "$rc" -eq 0 ]]; then
  echo "FAIL: expected runtime failure for div/mod by zero, but exited 0" >&2
  exit 1
fi

echo "runtime $name: PASS (exit=$rc)"
