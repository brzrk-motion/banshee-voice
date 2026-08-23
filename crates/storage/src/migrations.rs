pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[Migration {
    version: 1,
    name: "0001_initial",
    sql: include_str!("../migrations/0001_initial.sql"),
}];

pub fn all() -> &'static [Migration] {
    MIGRATIONS
}
