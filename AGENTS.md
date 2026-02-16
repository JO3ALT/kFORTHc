# Repository Guidelines

## Project Structure & Module Organization
- `src/main.rs`: Rust compiler frontend and LLVM IR code generator for the Forth-like language.
- `runtime/runtime.c`: C runtime functions (`pwrite_*`, `pread_*`, `pbool`, placeholder var/field helpers) linked with generated IR.
- `Cargo.toml`: Rust package metadata and build configuration.
- Keep new Rust modules under `src/` and runtime/native helpers under `runtime/`.

## Build, Test, and Development Commands
- `cargo build`: Compile the Rust compiler binary.
- `cargo run -- <input.fth> <output.ll>`: Parse/compile Forth source to LLVM IR.
- `cargo test`: Run Rust unit/integration tests (add tests as features evolve).
- `cargo fmt`: Apply Rust formatting.
- `cargo clippy -- -D warnings`: Lint Rust code and fail on warnings.
- Runtime build example:
  `clang runtime/runtime.c output.ll -o out && ./out`

## Coding Style & Naming Conventions
- Rust style follows `rustfmt` defaults (4-space indent, trailing commas where idiomatic).
- Use `snake_case` for Rust functions/variables and `CamelCase` for enums/types.
- Forth words and external service names are uppercase (for example `PWRITE-I32`, `MAIN`).
- Keep functions small and explicit; return `Result<_, String>` for user-facing compile errors.
- C runtime code should stay C99-compatible and use fixed-width integer types (`int32_t`).

## Testing Guidelines
- Prefer focused unit tests near parser/tokenizer/codegen logic in `src/main.rs` (`#[cfg(test)]`).
- Add integration tests under `tests/` for end-to-end compilation flows.
- Test names should describe behavior (`tokenize_string_literal`, `compile_if_else_then`).
- Validate both success paths and failure diagnostics for malformed input.

## Commit & Pull Request Guidelines
- No established history yet; adopt Conventional Commits (`feat:`, `fix:`, `test:`, `refactor:`).
- Keep commits scoped to one logical change.
- PRs should include:
  - concise summary and motivation,
  - test/verification commands run,
  - sample input/output when changing code generation or runtime behavior.
- Link related issues and call out breaking language/runtime changes explicitly.

## Security & Configuration Notes
- Treat `.fth` inputs as untrusted; preserve clear error handling and avoid unchecked assumptions.
- Do not hardcode machine-specific paths; keep commands repo-relative.
