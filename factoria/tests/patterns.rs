//! The usage patterns from the README (states, sequences, associations,
//! transactions per test), checked against a real database.

use factoria::Factory;
use sea_orm::{
    ColumnTrait, ConnectOptions, ConnectionTrait, Database, DatabaseConnection, EntityTrait,
    PaginatorTrait, QueryFilter, Schema, Set, TransactionTrait,
};

mod user {
    use sea_orm::entity::prelude::*;

    #[derive(Clone, Debug, PartialEq, Eq, DeriveEntityModel)]
    #[sea_orm(table_name = "users")]
    pub struct Model {
        #[sea_orm(primary_key)]
        pub id: i32,
        #[sea_orm(unique)]
        pub username: String,
        pub role: String,
        pub banned: bool,
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
            username: Set(format!("user_{seq}")),
            role: Set("member".into()),
            banned: Set(false),
            ..Default::default()
        }
    }
}

// States: named presets returning a pre-configured builder.
impl UserFactory {
    fn admin() -> factoria::Builder<Self> {
        Self::new().with(|u| u.role = Set("admin".into()))
    }

    fn banned() -> factoria::Builder<Self> {
        Self::new().with(|u| u.banned = Set(true))
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

// Associations: a helper that ties posts to an existing user.
impl PostFactory {
    fn for_user(user_id: i32) -> factoria::Builder<Self> {
        Self::new().with(move |p| p.user_id = Set(user_id))
    }
}

async fn setup() -> DatabaseConnection {
    let mut opts = ConnectOptions::new("sqlite::memory:");
    opts.max_connections(1);
    let db = Database::connect(opts).await.unwrap();
    let backend = db.get_database_backend();
    let schema = Schema::new(backend);
    db.execute(&schema.create_table_from_entity(user::Entity))
        .await
        .unwrap();
    db.execute(&schema.create_table_from_entity(post::Entity))
        .await
        .unwrap();
    db
}

#[tokio::test]
async fn states_apply_and_combine() {
    let db = setup().await;
    let admin = UserFactory::admin().create(&db).await.unwrap();
    assert_eq!(admin.role, "admin");
    assert!(!admin.banned);

    let banned = UserFactory::banned().create(&db).await.unwrap();
    assert!(banned.banned);

    let banned_admin = UserFactory::admin()
        .with(|u| u.banned = Set(true))
        .create(&db)
        .await
        .unwrap();
    assert_eq!(banned_admin.role, "admin");
    assert!(banned_admin.banned);
}

#[tokio::test]
async fn cloned_builder_is_reusable() {
    let db = setup().await;
    let base = UserFactory::admin();
    let a = base.clone().create(&db).await.unwrap();
    let b = base.create(&db).await.unwrap();
    assert_eq!((a.role.as_str(), b.role.as_str()), ("admin", "admin"));
    assert_ne!(a.username, b.username);
}

#[tokio::test]
async fn with_seq_gives_unique_values() {
    let db = setup().await;
    let users = UserFactory::new()
        .with_seq(|u, seq| u.username = Set(format!("player_{seq}")))
        .count(3)
        .create_many(&db)
        .await
        .unwrap();
    assert!(users.iter().all(|u| u.username.starts_with("player_")));
    let mut names: Vec<_> = users.iter().map(|u| u.username.clone()).collect();
    names.sort();
    names.dedup();
    assert_eq!(names.len(), 3);
}

#[tokio::test]
async fn associations_link_children_to_parent() {
    let db = setup().await;
    let author = UserFactory::new().create(&db).await.unwrap();

    // Inline, moving the parent's id into the closure.
    let author_id = author.id;
    PostFactory::new()
        .with(move |p| p.user_id = Set(author_id))
        .create(&db)
        .await
        .unwrap();

    // Through the helper.
    let posts = PostFactory::for_user(author.id)
        .count(3)
        .create_many(&db)
        .await
        .unwrap();
    assert!(posts.iter().all(|p| p.user_id == author.id));

    let total = post::Entity::find()
        .filter(post::Column::UserId.eq(author.id))
        .count(&db)
        .await
        .unwrap();
    assert_eq!(total, 4);
}

#[tokio::test]
async fn transaction_per_test_leaves_db_clean() {
    let db = setup().await;
    let txn = db.begin().await.unwrap();
    let user = UserFactory::new().create(&txn).await.unwrap();
    PostFactory::for_user(user.id)
        .count(2)
        .create_many(&txn)
        .await
        .unwrap();
    txn.rollback().await.unwrap();

    assert_eq!(user::Entity::find().count(&db).await.unwrap(), 0);
    assert_eq!(post::Entity::find().count(&db).await.unwrap(), 0);
}
