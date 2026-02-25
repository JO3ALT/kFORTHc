#!/usr/bin/env bash
set -euo pipefail

IR="required_words.ll"
OBJ="required_words.o"
BIN="required_words.out"
ACTUAL="required_words_actual.txt"
EXPECTED="required_words_expected.txt"

if command -v llc >/dev/null 2>&1; then
  LLC=llc
elif command -v llc-14 >/dev/null 2>&1; then
  LLC=llc-14
else
  echo "error: llc not found (tried: llc, llc-14)" >&2
  exit 1
fi

cargo build
./target/debug/kforthc required_words_test.fth "$IR"
"$LLC" -filetype=obj "$IR" -o "$OBJ"
clang -no-pie "$OBJ" runtime/runtime.c -o "$BIN" -lm
printf '42\n1\nK\n' | "./$BIN" > "$ACTUAL"

diff -u "$EXPECTED" "$ACTUAL"
echo "required words test: PASS"
