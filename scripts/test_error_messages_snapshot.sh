#!/usr/bin/env bash
set -euo pipefail

if [[ -n "${KPASCAL_BIN:-}" ]]; then
  :
elif command -v kpascal >/dev/null 2>&1; then
  KPASCAL_BIN="$(command -v kpascal)"
else
  KPASCAL_BIN="../kpascal/target/release/kpascal"
fi
NEG_DIR="samples/negative"
BUILD_DIR="samples/build"
mkdir -p "$BUILD_DIR"

if [[ ! -x "$KPASCAL_BIN" ]]; then
  echo "error: kpascal binary not found: $KPASCAL_BIN" >&2
  exit 1
fi

check_snapshot() {
  local name="$1"
  local src="$NEG_DIR/$name.pas"
  local actual="$BUILD_DIR/$name.stderr.actual"
  local expected="$NEG_DIR/$name.stderr.expected"

  if "$KPASCAL_BIN" < "$src" > "$BUILD_DIR/$name.fth" 2> "$actual"; then
    echo "FAIL: expected compile error but succeeded: $name" >&2
    return 1
  fi

  diff -u "$expected" "$actual"
  echo "snapshot $name: PASS"
}

check_snapshot "01_unknown_identifier"
check_snapshot "02_type_mismatch_assign"
check_snapshot "10_error_position_parse"
check_snapshot "11_error_position_include_parse"

echo "all error snapshots: PASS"
