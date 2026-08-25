pub struct Migration {
    pub version: i64,
    pub name: &'static str,
    pub sql: &'static str,
}

const MIGRATIONS: &[Migration] = &[
    Migration {
        version: 1,
        name: "0001_initial",
        sql: include_str!("../migrations/0001_initial.sql"),
    },
    Migration {
        version: 2,
        name: "0002_hud_background_defaults",
        sql: include_str!("../migrations/0002_hud_background_defaults.sql"),
    },
    Migration {
        version: 3,
        name: "0003_plugins",
        sql: include_str!("../migrations/0003_plugins.sql"),
    },
];

pub fn all() -> &'static [Migration] {
    MIGRATIONS
}
