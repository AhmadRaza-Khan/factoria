//! Async test data factories for Rust.
//!
//! Define a factory once, then create realistic rows in one line:
//!
//! ```ignore
//! let user = UserFactory::new().create(&db).await?;
//! let users = UserFactory::new().count(10).create_many(&db).await?;
//! let admin = UserFactory::new().with(|u| u.role = Set("admin".into())).create(&db).await?;
//! ```
//!
//! # Defining a factory
//!
//! Implement [`Factory`] for a zero-sized struct. [`Factory::definition`]
//! returns a fresh model with default values; `seq` is a counter unique to
//! that factory, so unique columns never collide.
//!
//! ```
//! use factoria::Factory;
//!
//! struct User {
//!     email: String,
//!     admin: bool,
//! }
//!
//! struct UserFactory;
//!
//! impl Factory for UserFactory {
//!     type Model = User;
//!
//!     fn definition(seq: u64) -> User {
//!         User { email: format!("user{seq}@test.dev"), admin: false }
//!     }
//! }
//!
//! let admin = UserFactory::new().with(|u| u.admin = true).build();
//! let users = UserFactory::new().count(3).build_many();
//! assert!(admin.admin);
//! assert_eq!(users.len(), 3);
//! ```
//!
//! # Saving to a database
//!
//! The [`Builder`] gains methods for each enabled feature:
//!
//! | Feature   | Adds                                                  |
//! |-----------|-------------------------------------------------------|
//! | `sea-orm` | `create()`, `create_many()` for SeaORM `ActiveModel`s |
//! | `sqlx`    | `SqlxInsert`, `insert()`, `insert_many()`             |
//! | `derive`  | `#[derive(Factory)]` with random data from `fake`     |
//!
//! With `sea-orm`, set the factory's `Model` to the entity's `ActiveModel`;
//! `create()` returns the saved entity `Model`. It takes any
//! `ConnectionTrait`, so a `DatabaseTransaction` works for rolling back
//! after each test.
//!
//! With `sqlx`, implement `SqlxInsert` for your model once to describe
//! the `INSERT`; `insert()` then saves built models.
#![cfg_attr(docsrs, feature(doc_cfg))]
#![warn(missing_docs)]

use std::any::TypeId;
use std::collections::HashMap;
use std::marker::PhantomData;
use std::sync::{Arc, Mutex, OnceLock};

#[cfg(feature = "sea-orm")]
mod sea_orm_adapter;

#[cfg(feature = "sqlx")]
mod sqlx_adapter;
#[cfg(feature = "sqlx")]
pub use sqlx_adapter::SqlxInsert;

/// Generates a `<Name>Factory` for a struct with named fields.
///
/// Fields marked `#[factory(default = "<expr>")]` use that expression,
/// which may refer to `seq`. Every other field gets a random value, so its
/// type must implement `fake::Dummy<fake::Faker>`.
///
/// ```
/// use factoria::Factory;
///
/// #[derive(Factory)]
/// struct User {
///     #[factory(default = "0")]
///     id: i64,
///     #[factory(default = "format!(\"user{seq}@test.dev\")")]
///     email: String,
///     name: String, // random
/// }
///
/// let user = UserFactory::new().with(|u| u.name = "Ahmad".into()).build();
/// assert_eq!(user.name, "Ahmad");
/// ```
///
/// Not supported yet: tuple structs, enums, generic structs.
#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use factoria_derive::Factory;

#[cfg(feature = "derive")]
#[doc(hidden)]
pub use fake; // used by generated code

/// Describes how to build a default model. Implement this for a zero-sized
/// "XxxFactory" struct, then call [`Factory::new`] to get a [`Builder`].
pub trait Factory: Sized + 'static {
    /// What the factory produces (a plain struct, or a SeaORM ActiveModel).
    type Model: Send + 'static;

    /// Default values. `seq` is unique per factory type, starting at 1.
    ///
    /// Counters are global and never reset, so don't assert on exact `seq`
    /// values; use them only to make columns unique.
    fn definition(seq: u64) -> Self::Model;

    /// Start a builder: `UserFactory::new()`.
    fn new() -> Builder<Self> {
        Builder::new()
    }
}

type Override<M> = Arc<dyn Fn(&mut M, u64) + Send + Sync>;

/// Configures and produces models: overrides, count, then build or save.
///
/// Builders are cheap to clone, so a configured one can be stored as a
/// preset ("state") and reused:
///
/// ```
/// # use factoria::{Builder, Factory};
/// # struct User { admin: bool }
/// # struct UserFactory;
/// # impl Factory for UserFactory {
/// #     type Model = User;
/// #     fn definition(_: u64) -> User { User { admin: false } }
/// # }
/// impl UserFactory {
///     fn admin() -> Builder<Self> {
///         Self::new().with(|u| u.admin = true)
///     }
/// }
///
/// assert!(UserFactory::admin().build().admin);
/// ```
pub struct Builder<F: Factory> {
    overrides: Vec<Override<F::Model>>,
    count: usize,
    _factory: PhantomData<fn() -> F>,
}

impl<F: Factory> Builder<F> {
    /// An empty builder: no overrides, count of 1.
    pub fn new() -> Self {
        Self {
            overrides: Vec::new(),
            count: 1,
            _factory: PhantomData,
        }
    }

    /// Override fields: `.with(|u| u.name = "Ahmad".into())`
    ///
    /// Overrides run after [`Factory::definition`], in the order added.
    /// The closure must be `'static`, so `move` in any values it uses.
    pub fn with(mut self, f: impl Fn(&mut F::Model) + Send + Sync + 'static) -> Self {
        self.overrides.push(Arc::new(move |m, _| f(m)));
        self
    }

    /// Override with access to the sequence number:
    /// `.with_seq(|u, seq| u.username = format!("player_{seq}"))`
    pub fn with_seq(mut self, f: impl Fn(&mut F::Model, u64) + Send + Sync + 'static) -> Self {
        self.overrides.push(Arc::new(f));
        self
    }

    /// How many models the `*_many` methods produce. Defaults to 1.
    pub fn count(mut self, n: usize) -> Self {
        self.count = n;
        self
    }

    /// Build one model in memory (no DB).
    pub fn build(&self) -> F::Model {
        let seq = next_seq::<F>();
        let mut model = F::definition(seq);
        for o in &self.overrides {
            o(&mut model, seq);
        }
        model
    }

    /// Build `count` models in memory, each with its own `seq`.
    pub fn build_many(&self) -> Vec<F::Model> {
        (0..self.count).map(|_| self.build()).collect()
    }
}

impl<F: Factory> Default for Builder<F> {
    fn default() -> Self {
        Self::new()
    }
}

impl<F: Factory> Clone for Builder<F> {
    fn clone(&self) -> Self {
        Self {
            overrides: self.overrides.clone(),
            count: self.count,
            _factory: PhantomData,
        }
    }
}

fn next_seq<F: 'static>() -> u64 {
    static COUNTERS: OnceLock<Mutex<HashMap<TypeId, u64>>> = OnceLock::new();
    let mut map = COUNTERS
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|e| e.into_inner());
    let n = map.entry(TypeId::of::<F>()).or_insert(0);
    *n += 1;
    *n
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct User {
        email: String,
        admin: bool,
    }

    struct UserFactory;
    impl Factory for UserFactory {
        type Model = User;
        fn definition(seq: u64) -> User {
            User {
                email: format!("user{seq}@test.dev"),
                admin: false,
            }
        }
    }

    struct OtherFactory;
    impl Factory for OtherFactory {
        type Model = u64;
        fn definition(seq: u64) -> u64 {
            seq
        }
    }

    #[test]
    fn builds_with_overrides_and_unique_seq() {
        let a = UserFactory::new().with(|u| u.admin = true).build();
        let b = UserFactory::new().build();
        assert!(a.admin);
        assert_ne!(a.email, b.email);
        assert_eq!(UserFactory::new().count(5).build_many().len(), 5);
    }

    #[test]
    fn overrides_run_in_order_after_definition() {
        let u = UserFactory::new()
            .with(|u| u.email = "first".into())
            .with_seq(|u, seq| u.email = format!("{}-{seq}", u.email))
            .build();
        assert!(u.email.starts_with("first-"));
    }

    #[test]
    fn clone_keeps_overrides_and_count() {
        let base = UserFactory::new().with(|u| u.admin = true).count(3);
        let users = base.clone().build_many();
        assert_eq!(users.len(), 3);
        assert!(users.iter().all(|u| u.admin));
    }

    #[test]
    fn seq_is_per_factory_type() {
        let first = OtherFactory::new().build();
        let second = OtherFactory::new().build();
        assert!(second > first);
    }

    #[test]
    fn builder_is_send_and_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<Builder<UserFactory>>();
    }
}
