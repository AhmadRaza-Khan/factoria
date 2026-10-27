#[derive(factoria::Factory)]
struct User {
    #[factory(default = 0)]
    id: i64,
}

fn main() {}
