use demo_app::app::App;
use loco_rs::testing::prelude::*;
use sea_orm::{ConnectionTrait, DatabaseBackend, Statement, Value};
use serial_test::serial;

/// RD-1 (specs/due-dates.md, T-01): a post created without a due date keeps a
/// NULL `due_date` in the database, and its representation reports it as null.
#[tokio::test]
#[serial]
async fn post_created_without_due_date_stores_null() {
    request::<App, _, _>(|request, ctx| async move {
        let res = request
            .post("/api/posts")
            .json(&serde_json::json!({ "title": "no due date" }))
            .await;

        assert_eq!(res.status_code(), 201, "create must succeed");
        let body = res.json::<serde_json::Value>();
        assert!(
            body["due_date"].is_null(),
            "the created post must report a null due date, got: {body}"
        );

        // Raw SQL on purpose: this asserts the stored column, independent of
        // the API and of entity codegen.
        let id = body["id"].as_i64().expect("post id");
        let stmt = Statement::from_sql_and_values(
            DatabaseBackend::Postgres,
            "SELECT due_date FROM posts WHERE id = $1",
            [Value::from(id)],
        );
        let row = ctx
            .db
            .query_one_raw(stmt)
            .await
            .expect("querying the stored post")
            .expect("the post row must exist");
        let stored = row
            .try_get::<Option<chrono::DateTime<chrono::FixedOffset>>>("", "due_date")
            .expect("reading the due_date column");

        assert!(
            stored.is_none(),
            "the stored due date must be NULL, got: {stored:?}"
        );
    })
    .await;
}
