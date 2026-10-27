use crate::{Builder, Factory};
use sqlx::{Database, Pool};
use std::future::Future;

/// Teach factoria how to insert your struct with sqlx.
///
/// Implementations may be written as `async fn insert(...)`; the compiler
/// checks that the returned future is `Send`.
#[cfg_attr(docsrs, doc(cfg(feature = "sqlx")))]
pub trait SqlxInsert<DB: Database>: Sized + Send {
    /// Insert and return the saved row (e.g. with the generated id).
    fn insert(self, pool: &Pool<DB>) -> impl Future<Output = Result<Self, sqlx::Error>> + Send;
}

#[cfg_attr(docsrs, doc(cfg(feature = "sqlx")))]
impl<F: Factory> Builder<F> {
    /// Insert one row.
    pub async fn insert<DB: Database>(&self, pool: &Pool<DB>) -> Result<F::Model, sqlx::Error>
    where
        F::Model: SqlxInsert<DB>,
    {
        SqlxInsert::insert(self.build(), pool).await
    }

    /// Insert `count` rows.
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
