#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${KPASCAL_BIN:-}" ]]; then
  :
elif command -v kpascal >/dev/null 2>&1; then
  KPASCAL_BIN="$(command -v kpascal)"
else
  KPASCAL_BIN="../kpascal/target/release/kpascal"
fi
PASCAL_SRC="pascal_tests/full_coverage.pas"
FORTH_OUT="pascal_tests/full_coverage.fth"
IR_OUT="pascal_tests/full_coverage.ll"
OBJ_OUT="pascal_tests/full_coverage.o"
BIN_OUT="pascal_tests/full_coverage.out"
ACTUAL_OUT="pascal_tests/full_coverage.actual"
EXPECTED_OUT="pascal_tests/full_coverage.expected"

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

cargo build
"$KPASCAL_BIN" < "$PASCAL_SRC" > "$FORTH_OUT"
./target/debug/kforthc "$FORTH_OUT" "$IR_OUT"
"$LLC" -filetype=obj "$IR_OUT" -o "$OBJ_OUT"
clang -no-pie "$OBJ_OUT" runtime/runtime.c -o "$BIN_OUT" -lm
printf '255\n1Z\n7 8 9\nHELLO\n' | "./$BIN_OUT" > "$ACTUAL_OUT"

diff -u "$EXPECTED_OUT" "$ACTUAL_OUT"
echo "kpascal full coverage test: PASS"
