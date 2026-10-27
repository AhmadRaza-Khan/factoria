//! A tour of factoria with SeaORM and in-memory SQLite.
//!
//! Run: cargo run -p factoria --example seaorm_basic --features sea-orm

use factoria::{Builder, Factory};
use sea_orm::{
    ConnectOptions, ConnectionTrait, Database, DatabaseConnection, DbErr, Schema, Set,
    TransactionTrait,
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
        pub role: String,
    }

    #[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
    pub enum Relation {}

    impl ActiveModelBehavior for ActiveModel {}
}

mod post {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "posts")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        pub user_id: i32,
        pub title: String,
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
            role: Set("member".into()),
            ..Default::default()
        }
    }
}

impl UserFactory {
    /// A state: a preset builder.
    fn admin() -> Builder<Self> {
        Self::new().with(|u| u.role = Set("admin".into()))
    }
}

struct PostFactory;

impl Factory for PostFactory {
    type Model = post::ActiveModel;

    fn definition(seq: u64) -> Self::Model {
        post::ActiveModel {
            title: Set(format!("Post {seq}")),
            ..Default::default()
        }
    }
}

impl PostFactory {
    /// An association helper: posts belonging to a user.
    fn for_user(user_id: i32) -> Builder<Self> {
        Self::new().with(move |p| p.user_id = Set(user_id))
    }
}

async fn connect() -> Result<DatabaseConnection, DbErr> {
    let mut opts = ConnectOptions::new("sqlite::memory:");
    opts.max_connections(1); // each in-memory SQLite connection is its own database
    let db = Database::connect(opts).await?;
    let schema = Schema::new(db.get_database_backend());
    db.execute(&schema.create_table_from_entity(user::Entity))
        .await?;
    db.execute(&schema.create_table_from_entity(post::Entity))
        .await?;
    Ok(db)
}

#[tokio::main]
async fn main() -> Result<(), DbErr> {
    let db = connect().await?;

    let user = UserFactory::new().create(&db).await?;
    println!("create:       {user:?}");

    let users = UserFactory::new().count(3).create_many(&db).await?;
    println!("create_many:  {} users", users.len());

    let ahmad = UserFactory::new()
        .with(|u| u.name = Set("Ahmad".into()))
        .create(&db)
        .await?;
    println!("override:     {ahmad:?}");

    let admin = UserFactory::admin().create(&db).await?;
    println!("state:        {admin:?}");

    let posts = PostFactory::for_user(admin.id)
        .count(2)
        .create_many(&db)
        .await?;
    println!("association:  {posts:?}");

    let txn = db.begin().await?;
    let temp = UserFactory::new().create(&txn).await?;
    txn.rollback().await?;
    println!("transaction:  created id {} then rolled back", temp.id);

    Ok(())
}
