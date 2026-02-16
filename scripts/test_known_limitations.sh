#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${KPASCAL_BIN:-}" ]]; then
  :
elif command -v kpascal >/dev/null 2>&1; then
  KPASCAL_BIN="$(command -v kpascal)"
else
  KPASCAL_BIN="../kpascal/target/release/kpascal"
fi
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

name="23_recursive_fib_constraint"
"$KPASCAL_BIN" < "$SAMPLES_DIR/$name.pas" > "$BUILD_DIR/$name.fth"
./target/debug/kforthc "$BUILD_DIR/$name.fth" "$BUILD_DIR/$name.ll"
"$LLC" -filetype=obj "$BUILD_DIR/$name.ll" -o "$BUILD_DIR/$name.o"
clang -no-pie "$BUILD_DIR/$name.o" runtime/runtime.c -o "$BUILD_DIR/$name.out"
"$BUILD_DIR/$name.out" > "$BUILD_DIR/$name.actual"

actual="$(tr -d '\r' < "$BUILD_DIR/$name.actual" | head -n 1 | tr -d '[:space:]')"
if [[ "$actual" != "8" ]]; then
  echo "FAIL: recursion regression detected (Fib(6) expected 8, got '$actual')." >&2
  exit 1
fi

echo "recursion fix check $name: PASS (Fib(6)=8)"
