use crate::{Builder, Factory};
use sea_orm::{
    ActiveModelBehavior, ActiveModelTrait, ConnectionTrait, DbErr, EntityTrait, IntoActiveModel,
};

type EntityModel<A> = <<A as ActiveModelTrait>::Entity as EntityTrait>::Model;

#[cfg_attr(docsrs, doc(cfg(feature = "sea-orm")))]
impl<F, A> Builder<F>
where
    F: Factory<Model = A>,
    A: ActiveModelTrait + ActiveModelBehavior + Send + 'static,
    EntityModel<A>: IntoActiveModel<A>,
{
    /// Insert one row and return the saved model.
    ///
    /// Accepts any `ConnectionTrait`, so a `DatabaseTransaction` works too:
    /// create rows inside a transaction and roll it back after the test.
    pub async fn create<C: ConnectionTrait>(&self, db: &C) -> Result<EntityModel<A>, DbErr> {
        self.build().insert(db).await
    }

    /// Insert `count` rows (one INSERT each, so ids/defaults come back),
    /// stopping at the first error.
    pub async fn create_many<C: ConnectionTrait>(
        &self,
        db: &C,
    ) -> Result<Vec<EntityModel<A>>, DbErr> {
        let mut out = Vec::with_capacity(self.count);
        for am in self.build_many() {
            out.push(am.insert(db).await?);
        }
        Ok(out)
    }
}
