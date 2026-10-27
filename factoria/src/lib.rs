//! Async test data factories for Rust.
#![cfg_attr(docsrs, feature(doc_cfg))]

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

#[cfg(feature = "derive")]
#[cfg_attr(docsrs, doc(cfg(feature = "derive")))]
pub use factoria_derive::Factory;

#[cfg(feature = "derive")]
#[doc(hidden)]
pub use fake; // used by generated code

/// Implement this for a zero-sized "XxxFactory" struct.
pub trait Factory: Sized + 'static {
    /// What the factory produces (a plain struct, or a SeaORM ActiveModel).
    type Model: Send + 'static;

    /// Default values. `seq` is unique per factory type, starting at 1.
    fn definition(seq: u64) -> Self::Model;

    /// Start a builder: `UserFactory::new()`.
    fn new() -> Builder<Self> {
        Builder::new()
    }
}

type Override<M> = Arc<dyn Fn(&mut M, u64) + Send + Sync>;

/// Configures and produces models.
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
    pub fn with(mut self, f: impl Fn(&mut F::Model) + Send + Sync + 'static) -> Self {
        self.overrides.push(Arc::new(move |m, _| f(m)));
        self
    }

    /// Override with access to the sequence number.
    pub fn with_seq(mut self, f: impl Fn(&mut F::Model, u64) + Send + Sync + 'static) -> Self {
        self.overrides.push(Arc::new(f));
        self
    }

    /// How many models build_many / create_many produce.
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

    /// Build `count` models in memory.
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
