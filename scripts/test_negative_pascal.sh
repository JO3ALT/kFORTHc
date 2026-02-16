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

expect_fail() {
  local src="$1"
  local pattern="$2"
  local name
  name="$(basename "$src")"
  local err="$BUILD_DIR/${name%.pas}.err"
  local out="$BUILD_DIR/${name%.pas}.fth"

  if "$KPASCAL_BIN" < "$src" > "$out" 2> "$err"; then
    echo "FAIL: expected compile error but succeeded: $name" >&2
    return 1
  fi

  if ! rg -q "$pattern" "$err"; then
    echo "FAIL: error message mismatch for $name" >&2
    echo "expected pattern: $pattern" >&2
    echo "actual stderr:" >&2
    cat "$err" >&2
    return 1
  fi

  echo "negative $name: PASS"
}

expect_fail "$NEG_DIR/01_unknown_identifier.pas" "unknown identifier"
expect_fail "$NEG_DIR/02_type_mismatch_assign.pas" "type|mismatch|assign"
expect_fail "$NEG_DIR/03_wrong_arg_count.pas" "argument|arity|parameter"
expect_fail "$NEG_DIR/04_bad_index_type.pas" "index|integer"
expect_fail "$NEG_DIR/05_parse_error_missing_end.pas" "parse error|expected"
expect_fail "$NEG_DIR/06_include_missing.pas" "include read failed|No such file|not found"
expect_fail "$NEG_DIR/07_include_cycle.pas" "include|cycle|recursion|read failed|too deep|stack"
expect_fail "$NEG_DIR/08_scoping_conflict.pas" "shadow|duplicate|already defined|conflict|same name"
expect_fail "$NEG_DIR/10_error_position_parse.pas" "line 3, column 3"
expect_fail "$NEG_DIR/11_error_position_include_parse.pas" "line 3, column 7"

echo "all negative tests: PASS"
