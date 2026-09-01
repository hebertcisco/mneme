# Repository guidelines

mneme is a Rust CLI that indexes a **user-provided** Markdown vault. This file is for contributors and coding agents working **in this repository**. It must stay free of machine-specific paths, personal names, emails, and private vault locations.

## Scope

Change source under `src/`, tests, `examples/`, and public docs. Do not commit:

- A real personal vault
- `<vault>/.mneme/` (SQLite, journal, locks)
- Secrets, tokens, or local absolute paths (`C:\Users\...`, `/home/<name>/...`)
- Dropbox or other sync-folder locations

The vault path is always injected at runtime (`--vault`, `MNEME_VAULT`, or the user config file). Defaults in code must stay generic.

## Architecture

| Module | Role |
|---|---|
| `src/main.rs` | Tracing + process exit |
| `src/cli.rs` | clap schema and command dispatch |
| `src/config.rs` | User/vault config; vault discovery |
| `src/atomic.rs` | Crash-safe write (tmp + fsync + rename) |
| `src/journal.rs` | Append-only JSONL operations log |
| `src/vault.rs` | Lock, walk notes, skip derived dirs |
| `src/note.rs` | Frontmatter, wikilinks, kind → folder |
| `src/index.rs` | SQLite WAL + FTS5 + activation |
| `src/recall.rs` | Ranked, token-budgeted retrieval |
| `src/encode.rs` | `remember` / update notes |
| `src/lang.rs` | Language tags and detection (no translation) |
| `src/graph.rs` | Export `graph/data.js` and mermaid |
| `src/consolidate.rs` | Decay / promotion pass |
| `src/doctor.rs` | Repair tmp, rebuild index, report conflicts |
| `src/compact.rs` | Token estimates and AI cards |

Markdown in the vault is source of truth. `.mneme/` is a hippocampus: disposable and rebuildable.

## Commands

```bash
cargo fmt
cargo test --all-targets
cargo build --release
cargo run -- --help
cargo run -- --vault examples/vault reindex
cargo run -- --vault examples/vault recall "index" --format ai --tokens 400
```

## Style

- rustfmt, 4-space indent, edition 2021
- `snake_case` items, `PascalCase` types, `SCREAMING_SNAKE_CASE` constants
- Library paths return `Result`; do not `unwrap` except in tests
- User-facing strings and contributor docs are English
- Do not hard-code a person's name as a wiki hub; use `INDEX` / configurable hubs
- New notes default to English. Source bodies in another language stay as written; tag `lang` (ISO 639-1)

## Tests

Add unit tests next to the code they cover (`#[cfg(test)]`). Use `tempfile` for vault fixtures. Cover atomic replace, frontmatter round-trip, wikilink extraction, and recall packing. Run `cargo test --all-targets` before considering a change done.

## Pull requests

- Imperative, specific commit messages (`Add WAL recovery to doctor`, not `fix stuff`)
- Summarize user-visible CLI changes and update README examples in the same PR
- Dual-license remains MIT OR Apache-2.0; do not add dependencies incompatible with that
