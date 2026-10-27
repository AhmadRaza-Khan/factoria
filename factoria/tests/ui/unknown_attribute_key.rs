#[derive(factoria::Factory)]
struct User {
    #[factory(defualt = "0")]
    id: i64,
}

fn main() {}
