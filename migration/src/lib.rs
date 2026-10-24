#![allow(elided_lifetimes_in_paths)]
#![allow(clippy::wildcard_imports)]
pub use sea_orm_migration::prelude::*;
mod m20220101_000001_users;

mod m20261009_110420_posts;
mod m20261024_100813_add_due_date_to_posts;
pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m20220101_000001_users::Migration),
            Box::new(m20261009_110420_posts::Migration),
            Box::new(m20261024_100813_add_due_date_to_posts::Migration),
            // inject-above (do not remove this comment)
        ]
    }
}
