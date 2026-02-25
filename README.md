# kforthc

[日本語版 README / Japanese README](README.ja.md)

`kforthc` is a Rust compiler that compiles **kFORTH programs** into **LLVM IR**, then to object/executable code via `llc` and `clang`.

- The primary target is kPascal-generated kFORTH IL; a practical standalone FORTH subset (including current float32-on-cell words) is also supported.
- It is also used as the backend for **kPascal** intermediate code.
- Practical pipeline: **kPascal -> kFORTH -> LLVM -> object/executable**.
- Because high-level words (including `:` definitions) are supported, `kforthc` can also be used as a standalone FORTH compiler for this subset.

## Getting Started (PATH-based)

```bash
which kpascal
cargo build
./scripts/test_samples.sh
```

If `which kpascal` does not return a path, add `kpascal` to your `PATH` first.

## Language Semantics

See `SPEC.md` for current agreed semantics (overflow/wrap, booleans, char width, runtime traps, uninitialized reads, etc.).
The exact preserved control structures are `IF/ELSE/THEN`, `BEGIN/UNTIL`, and `BEGIN/WHILE/REPEAT`.
Float support is FPU-oriented: normal finite-case behavior is the compatibility target, not strict bootstrap edge-case emulation.
The current subset also includes dictionary/data helpers (`HERE`, `ALLOT`, `CREATE`, `VARIABLE`, `,`, `CONSTANT`); note that `HERE/ALLOT` are runtime-managed while top-level layout is still computed at compile time (see `SPEC.md` for exact behavior/limits).

## Requirements

- Rust (`cargo`)
- LLVM `llc` (or `llc-14`)
- `clang`
- `kpascal` available on `PATH` (required for Pascal pipeline/tests)

## Build and Run (FORTH)

```bash
cargo build
./target/debug/kforthc example.fth out.ll
llc -filetype=obj out.ll -o out.o   # or llc-14
clang -no-pie out.o runtime/runtime.c -o a.out -lm
./a.out
```

Or use helper:

```bash
./scripts/build.sh
```

## kPascal Usage

This repository assumes `kpascal` is already available on `PATH`.

```bash
which kpascal
./scripts/test_kpascal_full.sh
```

## Samples and Tests

- Main sample suite (normal + edge):
  ```bash
  ./scripts/test_samples.sh
  ```
- Negative compiler tests:
  ```bash
  ./scripts/test_negative_pascal.sh
  ```
- Runtime failure checks (`div/mod 0`):
  ```bash
  ./scripts/test_runtime_failures.sh
  ```
- Recursion regression check:
  ```bash
  ./scripts/test_known_limitations.sh
  ```
- Error message snapshot tests:
  ```bash
  ./scripts/test_error_messages_snapshot.sh
  ```
- Required FORTH words test:
  ```bash
  ./scripts/test_required_words.sh
  ```

## Repository Layout

- `src/main.rs`: compiler core (tokenize/parse/codegen)
- `runtime/runtime.c`: runtime services used by generated code
- `samples/`: Pascal sample programs and expected outputs
- `scripts/`: build/test scripts
- `SPEC.md`: language semantics

## License

MIT. See `LICENSE`.
