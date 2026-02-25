# kforthc Language Spec (Current)

This document defines the current behavior expected by tests and runtime.

## Scope

- Primary source language: Pascal-like input compiled by `kpascal` to FORTH-like IL.
- This repository (`kforthc`) compiles that IL to LLVM IR and links with `runtime/runtime.c`.
- A standalone FORTH subset is also supported for development/debugging, but this is not a full self-hosting Forth system.

## Core Value Model

- Integer model is 32-bit signed (`i32`).
- Arithmetic uses wraparound semantics on overflow.
- Booleans are stored in one `i32` cell with `FALSE = 0`.
- Branching and boolean display (`PWRITE-BOOL`) treat any non-zero value as true.
- Compiler-generated integer comparisons and float predicate words currently return `-1`/`0`.
- `PBOOL` and `PREAD-BOOL` normalize to `1`/`0` (non-zero -> `1`, zero -> `0`).
- `char` is a 32-bit value (no implicit 8-bit narrowing).

## Arithmetic and Comparison

- `+`, `-`, `*`: wraparound `i32` arithmetic.
- `/` and `MOD` in generated IL map to signed division/remainder (`sdiv`, `srem`).
- `/MOD` is also supported (returns remainder then quotient).
- Division by zero is an accepted runtime trap (process termination).
- Comparisons produce `-1`/`0` (Pascal/Forth-style true/false values for generated comparisons).
- Additional bit/logic helpers supported by the current subset: `AND`, `OR`, `XOR`, `LSHIFT`, `RSHIFT`.

## Storage and Memory

- Variables/fields are accessed through runtime services (`PVAR@/PVAR!`, `PFIELD@/PFIELD!`).
- Addressing is byte-based at IL level; runtime resolves to 32-bit cells.
- Runtime memory is backed by a fixed global cell array (`MEM_CELLS`, currently 65536 cells).
- Runtime `HERE`/`ALLOT` operate on a byte-based runtime heap pointer.
- `HERE` returns the current runtime heap pointer (bytes).
- `ALLOT` advances the runtime heap pointer by the supplied byte count and clamps to the runtime memory range (`0 .. MEM_CELLS*4`).
- Program start resets the runtime heap pointer to the compiler-computed static data end (`rt_heap_reset(<top-level here>)`), so runtime allocations begin after top-level `CREATE`/`VARIABLE`/`,`/`ALLOT` layout.
- Top-level `CREATE`, `VARIABLE`, `,`, and `ALLOT` are still processed at compile time to compute static addresses/layout for generated IL.
- `CREATE` records the current compile-time layout pointer as the word's address.
- `VARIABLE` allocates one cell in compile-time layout (advances by 4 bytes).
- `,` allocates one 32-bit cell in compile-time layout (advances by 4 bytes); current implementation also expects a compile-time stack value and discards it while advancing.
- Top-level `ALLOT` and `CONSTANT` require a compile-time-resolvable value immediately before the word (literal, `HERE`, or an already-defined `CONSTANT`/`CREATE` symbol).
- Array bounds are not checked at runtime by design.
- Reads from uninitialized storage are defined by current implementation behavior and are part of the language spec.

## Control Flow and Calls

- Supported control flow in IL: `IF/ELSE/THEN`, `BEGIN/WHILE/REPEAT`, `BEGIN/UNTIL`.
- Return-stack words (`>R`, `R>`, `R@`) are supported.
- Recursive function calls are supported; branching recursion (`Fib`-style) is validated by tests.
- This is the intended control-structure set to preserve for standalone programming in this compiler.

## Strings and I/O

- `S" ..." PWRITE-STR` outputs string literals as generated.
- `S" ..."` also supports compile-time float parsing forms used by the current subset: `S" ..." READ-F32` and `S" ..." FNUMBER?`.
- Runtime I/O services include integer/boolean/char read/write plus line helpers.
- `PREAD-I32` / `PREAD-BOOL` / `PREAD-CHAR` / `PREAD-F32` are token-oriented (whitespace-delimited).
- `PREAD-CHAR` accepts either a single-character token or a numeric token; invalid input falls back to `0`.
- Invalid numeric input in `Read` follows current runtime fallback behavior (e.g., `0`).

## Supported Core Words (Standalone Subset)

- Stack: `DUP`, `DROP`, `SWAP`, `OVER`, `>R`, `R>`, `R@`
- Arithmetic/logic: `+`, `-`, `*`, `/`, `MOD`, `/MOD`, `NEGATE`, `AND`, `OR`, `XOR`, `LSHIFT`, `RSHIFT`
- Comparison: `=`, `<>`, `<`, `<=`, `>`, `>=`, `0=`, `0<`
- Control: `IF`, `ELSE`, `THEN`, `BEGIN`, `UNTIL`, `WHILE`, `REPEAT`
- Dictionary/data helpers used by generated IL: `HERE`, `CONSTANT`, `CREATE`, `VARIABLE`, `,`, `ALLOT`
- Runtime services: `PWRITE-*`, `PREAD-*`, `PVAR@/PVAR!`, `PFIELD@/PFIELD!`, `PBOOL`
- Common output aliases also supported: `.` (integer output), `EMIT` (char output)

## Float32-on-Cell Extension (Current)

- Float values are represented as IEEE754 `binary32` bit patterns in one 32-bit cell.
- This implementation is **FPU-oriented**: the compatibility target is normal finite-case behavior used by generated IL and basic standalone programs.
- Exact compatibility with `../kforth/bootstrap.fth` edge-case behavior (NaN/Inf propagation rules, special-case diagnostics, subnormal handling) is **not** guaranteed.
- Supported words include:
  `PREAD-F32`, `READ-F32`, `FNUMBER?`,
  `FADD`, `FSUB`, `FMUL`, `FDIV`,
  `FNEGATE`, `FABS`, `F=`, `F<`, `F<=`, `F0=`, `FZERO?`,
  `FINF?`, `FNAN?`, `FFINITE?`, `F+INF`, `F-INF`, `FNAN`,
  `S>F`, `F>S`, `Q16.16>F`, `F>Q16.16`, `FROUND-I32`,
  `F.`, `WRITE-F32`, `PWRITE-F32`.
- `PREAD-F32` accepts a following float token such as `0.125`, `2.5E+1`, `inf`, `-inf`, `nan`.
- Arithmetic words (`FADD`, `FSUB`, `FMUL`, `FDIV`) are currently implemented using host `float` operations in the runtime.
- NaN/Inf may be accepted and propagated according to host FPU / C runtime behavior.
- Float-to-integer style conversions (`F>S`, `FROUND-I32`, `F>Q16.16`) currently do not provide strict bootstrap-compatible diagnostics for NaN/Inf/overflow inputs.
- Normal finite-case examples (expected behavior):
  - `3 65536 * Q16.16>F 2 65536 * Q16.16>F FDIV F.` prints approximately `1.5000`
  - `PREAD-F32 2.5E+1 FROUND-I32 .` prints `25`
  - `S" -1.25e-1" READ-F32 IF F. THEN` prints approximately `-0.1250`
  - `S" xyz" FNUMBER?` returns `FALSE` (`0`)

## Error Behavior

- Compile-time parse/semantic errors include line/column in diagnostics.
- Some runtime faults (e.g., divide-by-zero) are expected to terminate execution.

## Conformance

Behavior is validated by:

- `scripts/test_samples.sh`
- `scripts/test_negative_pascal.sh`
- `scripts/test_runtime_failures.sh`
- `scripts/test_known_limitations.sh`
- `scripts/test_error_messages_snapshot.sh`
