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

cargo build

run_one() {
  local name="$1"
  local src="$SAMPLES_DIR/$name.pas"
  local expected="$SAMPLES_DIR/$name.expected"
  local forth="$BUILD_DIR/$name.fth"
  local ir="$BUILD_DIR/$name.ll"
  local obj="$BUILD_DIR/$name.o"
  local bin="$BUILD_DIR/$name.out"
  local actual="$BUILD_DIR/$name.actual"

  "$KPASCAL_BIN" < "$src" > "$forth"
  ./target/debug/kforthc "$forth" "$ir"
  "$LLC" -filetype=obj "$ir" -o "$obj"
  clang -no-pie "$obj" runtime/runtime.c -o "$bin"

  if [[ "$name" == "05_io_mix" ]]; then
    printf '42\n1Q\n7 8 9\nHELLO\n' | "$bin" > "$actual"
  elif [[ "$name" == "15_read_invalid_input" ]]; then
    printf 'abc\n' | "$bin" > "$actual"
  elif [[ "$name" == "26_readln_mixed" ]]; then
    printf '10 99\n20 30\nZ\n' | "$bin" > "$actual"
  else
    "$bin" > "$actual"
  fi

  diff -u "$expected" "$actual"
  echo "sample $name: PASS"
}

run_one "01_arith_loop"
run_one "02_record_array"
run_one "03_proc_func_case"
run_one "04_string_char_hex"
run_one "05_io_mix"
run_one "06_div_mod"
run_one "07_types_and_aliases"
run_one "08_record_copy_and_fields"
run_one "09_array3d_loop_sum"
run_one "10_var_params_and_types"
run_one "11_edge_i32_and_divmod"
run_one "12_edge_array_record_string"
run_one "13_bool_literals"
run_one "14_for_downto_case"
run_one "15_read_invalid_input"
run_one "17_oob_array_access"
run_one "18_recursive_factorial"
run_one "19_string_long_empty_escape"
run_one "20_for_post_value_spec"
run_one "21_oob_alias_behavior"
run_one "22_include_nested_success"
run_one "23_recursive_fib_constraint"
run_one "24_char_boundaries"
run_one "25_overflow_compare"
run_one "26_readln_mixed"
run_one "27_memory_model_layout"
run_one "28_char_range_semantics"
run_one "29_uninitialized_behavior"

echo "all samples: PASS"
