use chrono::SubsecRound;
use demo_app::{app::App, models::_entities::posts};
use loco_rs::testing::prelude::*;
use sea_orm::{
    ActiveModelTrait, ActiveValue::Set, ConnectionTrait, DatabaseBackend, Statement, Value,
};
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

/// RD-3 (specs/due-dates.md, T-03): a valid future `due_date` is persisted
/// and echoed on create, on update, and in the detail afterwards.
#[tokio::test]
#[serial]
async fn future_due_date_is_persisted_and_echoed_on_create_and_update() {
    request::<App, _, _>(|request, _ctx| async move {
        // Microsecond precision, as in the T-02 test: that is what the column
        // stores, so the round trip can be compared exactly.
        let due_2d = (chrono::Utc::now() + chrono::Duration::days(2))
            .naive_utc()
            .trunc_subsecs(6)
            .and_utc();
        let res = request
            .post("/api/posts")
            .json(&serde_json::json!({
                "title": "launch",
                "due_date": due_2d.to_rfc3339(),
            }))
            .await;

        assert_eq!(res.status_code(), 201, "create must succeed");
        let body = res.json::<serde_json::Value>();
        assert_eq!(
            rfc3339_utc(&body["due_date"], "the create response due_date"),
            due_2d,
            "the create response must echo the due date (RD-3)"
        );
        let id = body["id"].as_i64().expect("post id");

        let due_5d = (chrono::Utc::now() + chrono::Duration::days(5))
            .naive_utc()
            .trunc_subsecs(6)
            .and_utc();
        let res = request
            .put(&format!("/api/posts/{id}"))
            .json(&serde_json::json!({
                "title": "launch",
                "due_date": due_5d.to_rfc3339(),
            }))
            .await;

        assert_eq!(res.status_code(), 200, "update must succeed");
        let body = res.json::<serde_json::Value>();
        assert_eq!(
            rfc3339_utc(&body["due_date"], "the update response due_date"),
            due_5d,
            "the update response must echo the new due date (RD-3)"
        );

        let res = request.get(&format!("/api/posts/{id}")).await;
        assert_eq!(res.status_code(), 200, "fetching the post must succeed");
        let body = res.json::<serde_json::Value>();
        assert_eq!(
            rfc3339_utc(&body["due_date"], "the detail due_date"),
            due_5d,
            "the detail must return the updated due date (RD-3)"
        );
    })
    .await;
}

/// Read a JSON RFC 3339 timestamp as UTC. `what` names the field under test so
/// a failure points at the requirement it belongs to.
fn rfc3339_utc(value: &serde_json::Value, what: &str) -> chrono::DateTime<chrono::Utc> {
    value
        .as_str()
        .unwrap_or_else(|| panic!("{what} must be an RFC 3339 string, got: {value}"))
        .parse::<chrono::DateTime<chrono::FixedOffset>>()
        .unwrap_or_else(|e| panic!("{what} must parse as RFC 3339: {e}, got: {value}"))
        .with_timezone(&chrono::Utc)
}

/// RD-2 (specs/due-dates.md, T-02): the list and the detail representation
/// both carry `due_date` — the stored instant as an RFC 3339 timestamp when
/// the row has one.
#[tokio::test]
#[serial]
async fn post_representations_include_due_date() {
    request::<App, _, _>(|request, ctx| async move {
        // The API does not persist due_date until T-03, so this row is
        // inserted directly. Microsecond precision: that is what the column
        // stores, so the round trip can be compared exactly.
        let due_utc = (chrono::Utc::now() + chrono::Duration::days(2))
            .naive_utc()
            .trunc_subsecs(6)
            .and_utc();
        let seeded = posts::ActiveModel {
            title: Set("has a due date".to_string()),
            published: Set(Some(true)),
            due_date: Set(Some(due_utc.into())),
            ..Default::default()
        }
        .insert(&ctx.db)
        .await
        .expect("seed a post with a due date");

        let res = request.get("/api/posts").await;
        assert_eq!(res.status_code(), 200, "listing posts must succeed");
        let body = res.json::<serde_json::Value>();
        let items = body["items"].as_array().expect("items array").clone();
        assert!(!items.is_empty(), "the list must contain the seeded post");
        for post in &items {
            assert!(
                post.get("due_date").is_some(),
                "a listed post must carry a due_date field (RD-2), got: {post}"
            );
        }
        let listed = items
            .iter()
            .find(|p| p["id"] == serde_json::json!(seeded.id))
            .expect("the seeded post must be in the list");
        assert_eq!(
            rfc3339_utc(&listed["due_date"], "the listed due_date"),
            due_utc,
            "the list must echo the stored due date (RD-2)"
        );

        let res = request.get(&format!("/api/posts/{}", seeded.id)).await;
        assert_eq!(res.status_code(), 200, "fetching the post must succeed");
        let body = res.json::<serde_json::Value>();
        assert!(
            body.get("due_date").is_some(),
            "a fetched post must carry a due_date field (RD-2), got: {body}"
        );
        assert_eq!(
            rfc3339_utc(&body["due_date"], "the detail due_date"),
            due_utc,
            "the detail must echo the stored due date (RD-2)"
        );
    })
    .await;
}
