use factoria::Factory;

#[derive(Debug, factoria::Factory)]
struct User {
    #[factory(default = "0")]
    id: i64,
    #[factory(default = "format!(\"user{seq}@test.dev\")")]
    email: String,
    name: String,
    age: u8,
}

#[test]
fn derive_generates_factory() {
    let u = UserFactory::new().build();
    assert_eq!(u.id, 0);
    assert!(u.email.ends_with("@test.dev"));
    let _ = (u.name, u.age); // random but present
}

#[test]
fn derive_overrides() {
    let u = UserFactory::new().with(|u| u.age = 30).build();
    assert_eq!(u.age, 30);
}

#[test]
fn seq_in_default_makes_values_unique() {
    let users = UserFactory::new().count(3).build_many();
    assert_ne!(users[0].email, users[1].email);
    assert_ne!(users[1].email, users[2].email);
}

mod customers {
    use fake::faker::internet::en::SafeEmail;
    use fake::Fake;

    #[derive(Debug, factoria::Factory)]
    pub struct Customer {
        #[factory(default = "SafeEmail().fake()")]
        pub email: String,
        pub nickname: Option<String>,
    }
}

#[test]
fn faker_helpers_and_pub_factory() {
    let c = customers::CustomerFactory::new().build();
    assert!(c.email.contains('@'));
    let _ = c.nickname;
}

#[test]
fn invalid_input_gives_clear_errors() {
    let t = trybuild::TestCases::new();
    t.compile_fail("tests/ui/*.rs");
}
