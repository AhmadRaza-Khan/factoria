use factoria::Factory;
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait, PaginatorTrait,
    Schema, Set, TransactionTrait,
};

mod user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub name: String,
        #[sea_orm(unique)]
        pub email: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

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

async fn setup() -> DatabaseConnection {
    let mut opts = ConnectOptions::new("sqlite::memory:");
    opts.max_connections(1);
    let db = Database::connect(opts).await.unwrap();
    let backend = db.get_database_backend();
    let stmt = Schema::new(backend).create_table_from_entity(user::Entity);
    db.execute(&stmt).await.unwrap();
    db
}

#[tokio::test]
async fn create_inserts_one_row() {
    let db = setup().await;
    let u = UserFactory::new().create(&db).await.unwrap();
    assert!(u.id > 0);
}

#[tokio::test]
async fn create_many_respects_count_and_unique_emails() {
    let db = setup().await;
    let users = UserFactory::new().count(5).create_many(&db).await.unwrap();
    assert_eq!(users.len(), 5);
    let mut emails: Vec<_> = users.iter().map(|u| u.email.clone()).collect();
    emails.sort();
    emails.dedup();
    assert_eq!(emails.len(), 5);
}

#[tokio::test]
async fn overrides_apply() {
    let db = setup().await;
    let u = UserFactory::new()
        .with(|u| u.name = Set("Ahmad".into()))
        .create(&db)
        .await
        .unwrap();
    assert_eq!(u.name, "Ahmad");
}

#[tokio::test]
async fn create_works_inside_a_rolled_back_transaction() {
    let db = setup().await;
    let txn = db.begin().await.unwrap();
    UserFactory::new().count(2).create_many(&txn).await.unwrap();
    txn.rollback().await.unwrap();
    assert_eq!(user::Entity::find().count(&db).await.unwrap(), 0);
}
