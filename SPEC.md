# kforthc Language Spec (Current)

This document defines the current behavior expected by tests and runtime.

## Scope

- Source language: Pascal-like input compiled by `kpascal` to FORTH-like IL.
- This repository (`kforthc`) compiles that IL to LLVM IR and links with `runtime/runtime.c`.

## Core Value Model

- Integer model is 32-bit signed (`i32`).
- Arithmetic uses wraparound semantics on overflow.
- Boolean convention: `TRUE = -1`, `FALSE = 0`.
- `char` is a 32-bit value (no implicit 8-bit narrowing).

## Arithmetic and Comparison

- `+`, `-`, `*`: wraparound `i32` arithmetic.
- `/` and `MOD` in generated IL map to signed division/remainder (`sdiv`, `srem`).
- Division by zero is an accepted runtime trap (process termination).
- Comparisons produce Pascal boolean semantics via the runtime convention (`-1`/`0`).

## Storage and Memory

- Variables/fields are accessed through runtime services (`PVAR@/PVAR!`, `PFIELD@/PFIELD!`).
- Addressing is byte-based at IL level; runtime resolves to 32-bit cells.
- Array bounds are not checked at runtime by design.
- Reads from uninitialized storage are defined by current implementation behavior and are part of the language spec.

## Control Flow and Calls

- Supported control flow in IL: `IF/ELSE/THEN`, `BEGIN/WHILE/REPEAT`, `BEGIN/UNTIL`.
- Return-stack words (`>R`, `R>`, `R@`) are supported.
- Recursive function calls are supported; branching recursion (`Fib`-style) is validated by tests.

## Strings and I/O

- `S" ..." PWRITE-STR` outputs string literals as generated.
- Runtime I/O services include integer/boolean/char read/write plus line helpers.
- Invalid numeric input in `Read` follows current runtime fallback behavior (e.g., `0`).

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
