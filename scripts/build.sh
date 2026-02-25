#!/usr/bin/env bash
set -euo pipefail

INPUT="${1:-example.fth}"
IR="${2:-out.ll}"
OBJ="${3:-out.o}"
BIN="${4:-a.out}"

if command -v llc >/dev/null 2>&1; then
  LLC=llc
elif command -v llc-14 >/dev/null 2>&1; then
  LLC=llc-14
else
  echo "error: llc not found (tried: llc, llc-14)" >&2
  exit 1
fi

cargo build
./target/debug/kforthc "$INPUT" "$IR"
"$LLC" -filetype=obj "$IR" -o "$OBJ"
clang -no-pie "$OBJ" runtime/runtime.c -o "$BIN" -lm

"./$BIN"
