//! factoria with sqlx and in-memory SQLite.
//!
//! Run: cargo run -p factoria --example sqlx_basic --features sqlx

use factoria::{Factory, SqlxInsert};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Sqlite, SqlitePool};

#[derive(Debug)]
struct User {
    id: i64,
    name: String,
    email: String,
}

/// Tells factoria how to save a `User`. Written once, used by every test.
impl SqlxInsert<Sqlite> for User {
    async fn insert(self, pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        let id = sqlx::query("INSERT INTO users (name, email) VALUES (?, ?)")
            .bind(&self.name)
            .bind(&self.email)
            .execute(pool)
            .await?
            .last_insert_rowid();
        Ok(Self { id, ..self })
    }
}

struct UserFactory;

impl Factory for UserFactory {
    type Model = User;

    fn definition(seq: u64) -> User {
        User {
            id: 0,
            name: format!("User {seq}"),
            email: format!("user{seq}@test.dev"),
        }
    }
}

#[tokio::main]
async fn main() -> Result<(), sqlx::Error> {
    let pool = SqlitePoolOptions::new()
        .max_connections(1) // each in-memory SQLite connection is its own database
        .connect("sqlite::memory:")
        .await?;
    sqlx::query(
        "CREATE TABLE users (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            name TEXT NOT NULL,
            email TEXT NOT NULL UNIQUE
        )",
    )
    .execute(&pool)
    .await?;

    let user = UserFactory::new().insert(&pool).await?;
    println!("insert:       new id {} -> {user:?}", user.id);

    let users = UserFactory::new().count(3).insert_many(&pool).await?;
    println!("insert_many:  {users:?}");

    let ahmad = UserFactory::new()
        .with(|u| u.name = "Ahmad".into())
        .insert(&pool)
        .await?;
    println!("override:     {ahmad:?}");

    Ok(())
}
