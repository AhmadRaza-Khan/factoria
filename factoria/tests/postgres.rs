//! Runs against a real Postgres, so it's skipped by default. CI runs it with
//! a Postgres service; locally:
//!
//!     DATABASE_URL=postgres://... cargo test -p factoria --all-features --test postgres -- --ignored

use factoria::{Factory, SqlxInsert};
use sqlx::postgres::PgPoolOptions;
use sqlx::{PgPool, Postgres};

#[derive(Debug)]
struct User {
    id: i64,
    name: String,
}

// Same impl as the README's sqlx quick start.
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
        User {
            id: 0,
            name: format!("User {seq}"),
        }
    }
}

async fn setup() -> PgPool {
    let url = std::env::var("DATABASE_URL").expect("DATABASE_URL must point at Postgres");
    // One connection, so the TEMP table is visible to every query and
    // disappears when the test ends.
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&url)
        .await
        .unwrap();
    sqlx::query("CREATE TEMP TABLE users (id BIGSERIAL PRIMARY KEY, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[tokio::test]
#[ignore = "needs DATABASE_URL pointing at Postgres"]
async fn insert_and_insert_many_on_postgres() {
    let pool = setup().await;

    let user = UserFactory::new()
        .with(|u| u.name = "Ahmad".into())
        .insert(&pool)
        .await
        .unwrap();
    assert!(user.id > 0);
    assert_eq!(user.name, "Ahmad");

    let users = UserFactory::new()
        .count(3)
        .insert_many(&pool)
        .await
        .unwrap();
    assert_eq!(users.len(), 3);
    assert!(users.iter().all(|u| u.id > user.id));
}
