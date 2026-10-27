use factoria::{Factory, SqlxInsert};
use sqlx::sqlite::SqlitePoolOptions;
use sqlx::{Sqlite, SqlitePool};

#[derive(Debug, Clone)]
struct User {
    id: i64,
    name: String,
}

impl SqlxInsert<Sqlite> for User {
    async fn insert(self, pool: &SqlitePool) -> Result<Self, sqlx::Error> {
        let id = sqlx::query("INSERT INTO users (name) VALUES (?)")
            .bind(&self.name)
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
        }
    }
}

async fn setup() -> SqlitePool {
    let pool = SqlitePoolOptions::new()
        .max_connections(1)
        .connect("sqlite::memory:")
        .await
        .unwrap();
    sqlx::query("CREATE TABLE users (id INTEGER PRIMARY KEY AUTOINCREMENT, name TEXT NOT NULL)")
        .execute(&pool)
        .await
        .unwrap();
    pool
}

#[tokio::test]
async fn insert_returns_id() {
    let pool = setup().await;
    let user = UserFactory::new().insert(&pool).await.unwrap();
    assert!(user.id > 0);
}

#[tokio::test]
async fn insert_many_returns_ids() {
    let pool = setup().await;
    let users = UserFactory::new()
        .count(3)
        .insert_many(&pool)
        .await
        .unwrap();
    assert_eq!(users.len(), 3);
    assert!(users.iter().all(|u| u.id > 0));
}

#[tokio::test]
async fn overrides_are_persisted() {
    let pool = setup().await;
    let user = UserFactory::new()
        .with(|u| u.name = "Ahmad".into())
        .insert(&pool)
        .await
        .unwrap();
    let name: String = sqlx::query_scalar("SELECT name FROM users WHERE id = ?")
        .bind(user.id)
        .fetch_one(&pool)
        .await
        .unwrap();
    assert_eq!(name, "Ahmad");
}

#[tokio::test(flavor = "multi_thread")]
async fn insert_future_is_send() {
    let pool = setup().await;
    let handle = tokio::spawn(async move { UserFactory::new().insert(&pool).await });
    assert!(handle.await.unwrap().unwrap().id > 0);
}
