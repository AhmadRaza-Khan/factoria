# factoria

[![crates.io](https://img.shields.io/crates/v/factoria.svg)](https://crates.io/crates/factoria)
[![docs.rs](https://docs.rs/factoria/badge.svg)](https://docs.rs/factoria)
[![CI](https://github.com/AhmadRaza-Khan/factoria/actions/workflows/ci.yml/badge.svg)](https://github.com/AhmadRaza-Khan/factoria/actions)
[![license](https://img.shields.io/crates/l/factoria.svg)](#license)

Async, DB-aware test data factories for Rust, for SeaORM and sqlx, with
overrides, counts, unique sequences and an optional derive.

```rust
let user  = UserFactory::new().create(&db).await?;
let users = UserFactory::new().count(10).create_many(&db).await?;
let admin = UserFactory::admin().with(|u| u.name = Set("Root".into())).create(&db).await?;
```

## Why

Tests that need database rows tend to fill up with hand-written `INSERT`s
and 15-field struct literals, repeated in every test and broken by every
new column. With factoria you describe a valid row once, then each test
creates what it needs in one line and overrides only the fields it cares
about.

## Install

```toml
[dev-dependencies]
factoria = { version = "0.1", features = ["sea-orm"] }   # or "sqlx", "derive"
```

factoria doesn't pick a database driver or async runtime for you: enable
those on `sea-orm` / `sqlx` in your own `Cargo.toml` as usual.

## Quick start (SeaORM)

The factory's `Model` is your entity's `ActiveModel`. Leave the primary key
out so the database assigns it.

```rust
use factoria::Factory;
use sea_orm::Set;

struct UserFactory;

impl Factory for UserFactory {
    type Model = user::ActiveModel;

    fn definition(seq: u64) -> Self::Model {
        user::ActiveModel {
            name: Set(format!("User {seq}")),
            email: Set(format!("user{seq}@test.dev")),
            ..Default::default()
        }
    }
}

let user = UserFactory::new().create(&db).await?;   // returns user::Model, with id
let users = UserFactory::new().count(5).create_many(&db).await?;
```

## Quick start (sqlx)

sqlx has no ORM insert, so tell factoria how to save your struct once by
implementing `SqlxInsert`. It works with Postgres, MySQL and SQLite.

```rust
use factoria::{Factory, SqlxInsert};
use sqlx::{PgPool, Postgres};

struct User { id: i64, name: String }

impl SqlxInsert<Postgres> for User {
    async fn insert(self, pool: &PgPool) -> Result<Self, sqlx::Error> {
        let id = sqlx::query_scalar("INSERT INTO users (name) VALUES ($1) RETURNING id")
            .bind(&self.name)
            .fetch_one(pool)
            .await?;
        Ok(Self { id, ..self })
    }
}

struct UserFactory;

impl Factory for UserFactory {
    type Model = User;
    fn definition(seq: u64) -> User {
        User { id: 0, name: format!("User {seq}") }
    }
}

let user = UserFactory::new().insert(&pool).await?;
let users = UserFactory::new().count(5).insert_many(&pool).await?;
```

## Derive for plain structs

With the `derive` feature, `#[derive(Factory)]` generates `<Name>Factory`.
Fields with `#[factory(default = "...")]` use that expression (`seq` is in
scope); every other field gets a random value from [`fake`](https://crates.io/crates/fake).

```rust
use factoria::Factory;

#[derive(Debug, Factory)]
struct User {
    #[factory(default = "0")]
    id: i64,
    #[factory(default = "format!(\"user{seq}@test.dev\")")]
    email: String,
    name: String, // random
    age: u8,      // random
}

let user = UserFactory::new().with(|u| u.age = 30).build();
```

Fields without a `default` must implement `fake::Dummy<fake::Faker>`
(strings, numbers, `bool`, `Option<T>`, `Vec<T>`, and more with `fake`'s
`chrono` / `uuid` features). To use faker helpers in defaults, add `fake`
to your own dev-dependencies:

```rust
use fake::Fake;
use fake::faker::internet::en::SafeEmail;

#[derive(Factory)]
struct Customer {
    #[factory(default = "SafeEmail().fake()")]
    email: String,
}
```

## Overrides, counts, sequences

```rust
// Overrides run after definition(), in the order added.
let user = UserFactory::new()
    .with(|u| u.name = Set("Ahmad".into()))
    .create(&db)
    .await?;

// Count applies to build_many / create_many / insert_many.
let users = UserFactory::new().count(10).create_many(&db).await?;

// with_seq gives the override the row's sequence number.
let players = UserFactory::new()
    .with_seq(|u, seq| u.username = Set(format!("player_{seq}")))
    .count(3)
    .create_many(&db)
    .await?;

// No database needed: build in memory.
let model = UserFactory::new().build();
let models = UserFactory::new().count(3).build_many();
```

`seq` starts at 1 and is unique per factory type. Counters are global and
never reset, so parallel tests never collide on unique columns. Don't
assert on exact `seq` values.

## States

Named presets are plain functions returning a configured `Builder`:

```rust
impl UserFactory {
    pub fn admin() -> factoria::Builder<Self> {
        Self::new().with(|u| u.role = Set("admin".into()))
    }

    pub fn banned() -> factoria::Builder<Self> {
        Self::new().with(|u| u.banned = Set(true))
    }
}

let admin = UserFactory::admin().create(&db).await?;
let banned_admin = UserFactory::admin()
    .with(|u| u.banned = Set(true))
    .create(&db)
    .await?;
```

Builders are `Clone`, so you can keep one around and reuse it:
`let base = UserFactory::admin(); base.clone().create(&db).await?;`

## Associations

Create the parent, then pass its id into the child. Closures must be
`'static`, so copy the id out or use `move`:

```rust
let author = UserFactory::new().create(&db).await?;
let author_id = author.id;

let posts = PostFactory::new()
    .with(move |p| p.user_id = Set(author_id))
    .count(3)
    .create_many(&db)
    .await?;
```

Or add a helper on the child factory:

```rust
impl PostFactory {
    pub fn for_user(user_id: i32) -> factoria::Builder<Self> {
        Self::new().with(move |p| p.user_id = Set(user_id))
    }
}

let posts = PostFactory::for_user(author.id).count(3).create_many(&db).await?;
```

## Transactions per test

`create()` accepts any SeaORM `ConnectionTrait`, including a transaction,
so each test can roll back and leave the database clean:

```rust
use sea_orm::TransactionTrait;

let txn = db.begin().await?;
let user = UserFactory::new().create(&txn).await?;
// ... assertions ...
txn.rollback().await?;
```

## Feature flags

| Feature   | Enables                                         |
|-----------|-------------------------------------------------|
| `sea-orm` | `create()`, `create_many()`                     |
| `sqlx`    | `SqlxInsert`, `insert()`, `insert_many()`       |
| `derive`  | `#[derive(Factory)]` with random data from fake |

No features are on by default; `build()` and `build_many()` always work.

## Examples

```sh
cargo run -p factoria --example seaorm_basic --features sea-orm
cargo run -p factoria --example sqlx_basic --features sqlx
```

## MSRV

Rust 1.94 (required by SeaORM 2 and sqlx 0.9).

## Limitations and roadmap

- `#[derive(Factory)]` supports structs with named fields only, without
  generics.
- The derive doesn't work on SeaORM `ActiveModel`s; implement `Factory` by
  hand for those (see the SeaORM quick start).
- `create_many` / `insert_many` run one `INSERT` per row so every row comes
  back with its id. That's fine for tests, not for bulk seeding.

Planned: a Diesel adapter, derive support for SeaORM entities, automatic
associations, `after_create` hooks, locale-aware fake data, and
transaction-per-test helpers.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

Unless you explicitly state otherwise, any contribution intentionally
submitted for inclusion in factoria by you, as defined in the Apache-2.0
license, shall be dual licensed as above, without any additional terms or
conditions.
