# Changelog

All notable changes to this project are documented here. The format is
based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/), and this
project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

`factoria` and `factoria-derive` are always released together with the
same version.

## [Unreleased]

First release, planned as 0.1.0.

### Added

- `Factory` trait and `Builder` with `with`, `with_seq`, `count`, `build`
  and `build_many`; sequence numbers are unique per factory type.
- SeaORM adapter (feature `sea-orm`, SeaORM 2): `create` and `create_many`,
  accepting any `ConnectionTrait` including transactions.
- sqlx adapter (feature `sqlx`, sqlx 0.9): `SqlxInsert` trait, `insert`
  and `insert_many`, for Postgres, MySQL and SQLite.
- `#[derive(Factory)]` (feature `derive`) for structs with named fields,
  with random values from `fake` 5 and `#[factory(default = "...")]`.
- Minimum supported Rust version: 1.94.

[Unreleased]: https://github.com/AhmadRaza-Khan/factoria/commits/master
