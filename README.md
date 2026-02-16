# kforthc

`kforthc` is a small Rust compiler that translates a subset of FORTH-like IL into LLVM IR text.
The generated IR is assembled by `llc` and linked with `runtime/runtime.c` by `clang`.

## Requirements

- Rust toolchain (`cargo`)
- LLVM `llc` (or `llc-14`)
- `clang`

## Quick Start

```bash
./scripts/build.sh
```

This runs:
1. `cargo build`
2. `./target/debug/kforthc example.fth out.ll`
3. `llc -filetype=obj out.ll -o out.o` (or `llc-14`)
4. `clang -no-pie out.o runtime/runtime.c -o a.out`
5. `./a.out`

## Manual Build

```bash
cargo build
./target/debug/kforthc example.fth out.ll
llc-14 -filetype=obj out.ll -o out.o
clang -no-pie out.o runtime/runtime.c -o a.out
./a.out
```

## Example Program

`example.fth` includes:
- arithmetic (`3 4 +`)
- control flow (`IF/ELSE/THEN`)
- string output (`S" ... " PWRITE-STR`)

Expected output:

```text
 sum: 7
 if:  true
```
