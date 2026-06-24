# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Commands

```bash
cargo build              # debug build
cargo build --release    # release build (LTO enabled)
cargo test               # run all tests
cargo test <test_name>   # run a single test, e.g. cargo test test_int
cargo run -- <path.avdl> <output.avpr>  # run the CLI
```

## Architecture

This is a Rust CLI that converts Avro IDL (`.avdl`) files to Avro Protocol (`.avpr`) JSON. The pipeline is:

```
.avdl text → AvroIdlLexer → RawField AST → LinkParser → Field AST → AvprSerializer → JSON
```

**`src/ast.rs`** — Two parallel AST enums:
- `RawField`: output of the lexer. Cross-references between records/enums are `RawField::Unresolved(name, type_name, docstring)` until linking.
- `Field`: output of the linker. `Unresolved` variants are replaced with `Field::RecordReference` or `Field::EnumReference` after the type is looked up in the protocol.
- `HasDefault<T>`: a tri-state (`Default(Some(v))` / `Default(None)` for null / `None` for no default set). The codebase has TODOs noting this should eventually become `Option<T>` for non-nullable fields.

**`src/lexer.rs`** — `AvroIdlLexer` uses the `chumsky` 0.9 parser-combinator library to build the `RawField` AST from source text. It also handles `import idl "path"` by recursively calling `parse_idl` on the imported file. The protocol's namespace annotation is propagated to all child records and enums during this phase.

**`src/linker.rs`** — `LinkParser` resolves `RawField::Unresolved` nodes by looking up the referenced type name in the top-level protocol via `find_field_by_name`. Records resolve to `Field::RecordReference`, enums resolve to `Field::EnumReference` (inheriting the enum's default).

**`src/serializer.rs`** — `AvprSerializer` walks the `Field` AST and produces a `serde_json::Value` written to the output file. Only `.avpr` output is implemented; `.avsc` is stubbed out.

**`src/runner.rs`** — `AvroIdlParser` wires the three stages together.

**`src/main.rs`** — CLI entry point using `clap` derive. Args: `<PATH> <OUTPUT_PATH> [FORMAT]`.

## Known limitations

- Array fields must be the first field in a record, or must have a docstring, otherwise parsing fails (known bug).
- Only `import idl` is supported; `import avsc` and `import avpr` are not.
- `//` line comments are not parsed; only `/** ... */` docstrings are supported.
- Nullable shorthand (`int?`) creates a `union{T, null}` internally.
- Tests live in `#[cfg(test)]` blocks at the bottom of `lexer.rs` and `linker.rs`.
