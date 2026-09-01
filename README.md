<p align="center">
  <img src="art/mnemosine.png" alt="Mnemosyne, Greek goddess of memory, holding a stylus and a scroll" width="320">
</p>

# mneme

Crash-safe **shared memory engine** for a Markdown vault. Windows, Linux, and macOS.

Markdown notes are the source of truth (the cortex). mneme keeps a rebuildable index under `<vault>/.mneme/` (the hippocampus): atomic writes, a journal, full-text search, ACT-R style activation, spreading-activation recall with a token budget, and graph export.

Durable notes, CLI output, and documentation in this repository are **English**.

## Features

- Atomic note writes (temp file, fsync, rename) under an exclusive vault lock
- JSONL journal for crash recovery
- SQLite FTS5 index that can be rebuilt from `.md` files alone
- Ranked `recall` cards so agents do not dump the whole vault into context
- Wikilink graph export (`graph/data.js` and mermaid in `GRAPH.md`)
- Consolidation / decay pass modeled on complementary learning systems

## Install

```bash
cargo install --path .
```

Requires a recent stable Rust toolchain (1.74+).

## Quick start

Point mneme at **your** vault. Do not commit that path.

```bash
export MNEME_VAULT="/path/to/your/vault"   # Unix
set MNEME_VAULT=D:\vault                   # Windows cmd
```

Or pass it every time:

```bash
mneme --vault /path/to/your/vault reindex
mneme --vault /path/to/your/vault recall "preferences" --format ai --tokens 700
```

You can also set `vault` in the user config file:

- Linux: `~/.config/mneme/config.toml`
- macOS: `~/Library/Application Support/mneme/config.toml`
- Windows: `%APPDATA%\mneme\config.toml`

```toml
vault = "/path/to/your/vault"
language = "en"
```

If `MNEME_VAULT` is unset and the current directory contains `INDEX.md`, that directory is used. Otherwise mneme exits and asks you to pass `--vault` or set `MNEME_VAULT`. A repository that only has a contributor `AGENTS.md` is not treated as a vault.

### Commands

```text
mneme init
mneme reindex
mneme recall "cue" --tokens 700 --format ai
mneme remember --kind memory --title "Title" --body "..."
mneme graph
mneme consolidate
mneme doctor
mneme status
mneme show Facts
mneme context
```

## Vault layout (convention)

mneme does not require this layout, but recall and graph work best when notes are atomic, English, and linked with `[[wikilinks]]`:

```text
vault/
  INDEX.md
  AGENTS.md
  GRAPH.md
  00-system/
  01-identity/
  02-memory/
  03-projects/
  04-entities/
  05-journal/
  06-sdd/
  07-agents/
  graph/          # HTML visualization (generated)
  .mneme/         # derived index — do not treat as source of truth
```

`.mneme/` is local derived state. Rebuild it with `mneme reindex`. Do not publish it.

A tiny example vault lives in [`examples/vault`](examples/vault).

## Development

```bash
cargo fmt
cargo test
cargo build --release
cargo run -- --help
```

See [AGENTS.md](AGENTS.md) for contributor conventions and [CONTRIBUTING.md](CONTRIBUTING.md) for pull requests.

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE))
- MIT license ([LICENSE-MIT](LICENSE-MIT))

at your option.
