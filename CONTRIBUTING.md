# Contributing to factoria

Thanks for helping out. Bug reports, docs fixes and small PRs are welcome
any time. For a new feature or a bigger change, please open an issue first
so we can agree on the approach before you write the code.

## Running the tests

```sh
cargo test --all-features
```

Tests use in-memory SQLite, so no database setup is needed. Before opening
a PR, also run:

```sh
cargo fmt --check
cargo clippy --workspace --all-features --all-targets
RUSTDOCFLAGS="-D warnings" cargo doc --all-features --no-deps
```

The minimum supported Rust version is 1.94. To check it:

```sh
rustup toolchain install 1.94.0
cargo +1.94.0 test --all-features
```

## Derive macro error messages

The compile-fail cases in `factoria/tests/ui/` check the derive macro's
error messages. If you change a message on purpose, regenerate the expected
output and commit the updated `.stderr` files:

```sh
TRYBUILD=overwrite cargo test -p factoria --features derive --test derive
```

## Commit messages

Use [Conventional Commits](https://www.conventionalcommits.org/); the
changelog is generated from them:

- `feat: add after_create hooks`
- `fix: keep sequence counter after a panicking test`
- `docs: explain transactions per test`
- `chore: bump sqlx to 0.9.1`

## License

By contributing, you agree that your contributions are licensed under
MIT OR Apache-2.0, the same as the project.
