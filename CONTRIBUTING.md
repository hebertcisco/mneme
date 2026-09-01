# Contributing

Thanks for contributing to mneme.

## How to work

1. Fork and branch from `main`.
2. Keep the vault path out of the repo. Use `--vault examples/vault` or `MNEME_VAULT` in your own shell.
3. `cargo fmt && cargo test --all-targets`
4. Open a pull request that explains *why*, not only *what*.

## Rules

- Public docs (`README.md`, `AGENTS.md`, `examples/`) must not contain personal machine paths, private emails, or anyone's home directory.
- Do not commit `.mneme/`, SQLite files, or a real memory vault.
- New user-facing text is English. Vault note bodies may stay in another language when tagged with `lang`.
- License: contributions are dual-licensed MIT OR Apache-2.0 (see `LICENSE-MIT` and `LICENSE-APACHE`).

## Security

See [SECURITY.md](SECURITY.md).
