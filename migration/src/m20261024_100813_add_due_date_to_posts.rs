use loco_rs::schema::*;
use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    // RD-1 (specs/due-dates.md): due dates are optional, so the column is
    // nullable — existing rows get NULL.
    async fn up(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            alter("posts")
                .add_column(timestamptz_null("due_date"))
                .to_owned(),
        )
        .await
    }

    async fn down(&self, m: &SchemaManager) -> Result<(), DbErr> {
        m.alter_table(
            alter("posts")
                .drop_column(Alias::new("due_date"))
                .to_owned(),
        )
        .await
    }
}
