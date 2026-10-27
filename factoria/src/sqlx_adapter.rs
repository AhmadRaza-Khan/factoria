use crate::{Builder, Factory};
use sqlx::{Database, Pool};
use std::future::Future;

/// Teach factoria how to insert your struct with sqlx.
///
/// Implementations may be written as `async fn insert(...)`; the compiler
/// checks that the returned future is `Send`.
///
/// ```no_run
/// use factoria::{Factory, SqlxInsert};
/// use sqlx::{Sqlite, SqlitePool};
///
/// struct User {
///     id: i64,
///     name: String,
/// }
///
/// impl SqlxInsert<Sqlite> for User {
///     async fn insert(self, pool: &SqlitePool) -> Result<Self, sqlx::Error> {
///         let id = sqlx::query("INSERT INTO users (name) VALUES (?)")
///             .bind(&self.name)
///             .execute(pool)
///             .await?
///             .last_insert_rowid();
///         Ok(Self { id, ..self })
///     }
/// }
///
/// struct UserFactory;
///
/// impl Factory for UserFactory {
///     type Model = User;
///
///     fn definition(seq: u64) -> User {
///         User { id: 0, name: format!("User {seq}") }
///     }
/// }
///
/// # async fn run(pool: SqlitePool) -> Result<(), sqlx::Error> {
/// let user = UserFactory::new().insert(&pool).await?;
/// let users = UserFactory::new().count(3).insert_many(&pool).await?;
/// # Ok(())
/// # }
/// ```
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx")))]
pub trait SqlxInsert<DB: Database>: Sized + Send {
    /// Insert and return the saved row (e.g. with the generated id).
    fn insert(self, pool: &Pool<DB>) -> impl Future<Output = Result<Self, sqlx::Error>> + Send;
}

#[cfg_attr(docsrs, doc(cfg(feature = "sqlx")))]
impl<F: Factory> Builder<F> {
    /// Build one model and insert it with [`SqlxInsert::insert`].
    pub async fn insert<DB: Database>(&self, pool: &Pool<DB>) -> Result<F::Model, sqlx::Error>
    where
        F::Model: SqlxInsert<DB>,
    {
        SqlxInsert::insert(self.build(), pool).await
    }

    /// Build and insert `count` models, one INSERT each, stopping at the
    /// first error.
    pub async fn insert_many<DB: Database>(
        &self,
        pool: &Pool<DB>,
    ) -> Result<Vec<F::Model>, sqlx::Error>
    where
        F::Model: SqlxInsert<DB>,
    {
        let mut out = Vec::with_capacity(self.count);
        for m in self.build_many() {
            out.push(SqlxInsert::insert(m, pool).await?);
        }
        Ok(out)
    }
}
