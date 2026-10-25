use demo_app::models::_entities::posts;
use sea_orm::{ActiveModelTrait, ActiveValue::Set};
use serde_json::{json, Value};

use crate::PostsWorld;
use cucumber::{given, then, when};

impl PostsWorld {
    fn server(&self) -> &axum_test::TestServer {
        self.server.as_ref().expect("the posts API is not running")
    }

    fn record_response(&mut self, res: axum_test::TestResponse) {
        self.last_status = Some(res.status_code().as_u16());
        let text = res.text();
        self.last_body = serde_json::from_str(&text).ok();
    }

    /// The `items` array of the last list response (empty when there is none).
    fn list_items(&self) -> Vec<Value> {
        self.last_body
            .as_ref()
            .and_then(|b| b["items"].as_array())
            .cloned()
            .unwrap_or_default()
    }
}

#[given(expr = "the posts table contains a published post due in {int} days")]
async fn seed_published_post_due_in_days(world: &mut PostsWorld, days: i64) {
    // The precondition is a table state, so it is inserted directly (see the
    // seeding note in the feature): the API cannot store a due date until
    // T-03, and Q8 would refuse past dates anyway.
    let ctx = world.ctx.as_ref().expect("the posts API is not running");
    let due: sea_orm::prelude::DateTimeWithTimeZone =
        (chrono::Utc::now() + chrono::Duration::days(days)).into();
    let item = posts::ActiveModel {
        title: Set(format!("seed published post due in {days} days")),
        published: Set(Some(true)),
        due_date: Set(Some(due)),
        ..Default::default()
    }
    .insert(&ctx.db)
    .await
    .expect("seeding the published post");
    world.last_post = Some((item.id, item.title));
}

#[when(regex = r#"Anna creates a post titled "(.+)" with a due date (.+)"#)]
async fn create_post_with_due_date(world: &mut PostsWorld, title: String, due_date: String) {
    let res = world
        .server()
        .post("/api/posts")
        .json(&json!({ "title": title, "due_date": due_date }))
        .await;
    world.record_response(res);
    if world.last_status == Some(201) {
        let body = world
            .last_body
            .clone()
            .expect("create response must be JSON");
        world.last_post = Some((body["id"].as_i64().expect("post id"), title.to_string()));
    }
}

#[when(regex = r#"Anna creates a post titled "(.+)" without a due date"#)]
async fn create_post_without_due_date(world: &mut PostsWorld, title: String) {
    let res = world
        .server()
        .post("/api/posts")
        .json(&json!({ "title": title }))
        .await;
    world.record_response(res);
    if world.last_status == Some(201) {
        let body = world
            .last_body
            .clone()
            .expect("create response must be JSON");
        world.last_post = Some((body["id"].as_i64().expect("post id"), title));
    }
}

#[then("the created post has no due date")]
fn created_post_has_no_due_date(world: &mut PostsWorld) {
    let body = world
        .last_body
        .as_ref()
        .expect("create response must be JSON");
    assert!(
        body["due_date"].is_null(),
        "the created post must have a null due date, got: {body}"
    );
}

#[then("the stored post has no due date")]
async fn stored_post_has_no_due_date(world: &mut PostsWorld) {
    let ctx = world.ctx.as_ref().expect("the posts API is not running");
    let (id, _) = world.last_post.clone().expect("no post was created");
    // Raw SQL on purpose: this asserts the stored column, independent of the
    // API and the entity codegen.
    let stmt = sea_orm::Statement::from_sql_and_values(
        sea_orm::DatabaseBackend::Postgres,
        "SELECT due_date FROM posts WHERE id = $1",
        [sea_orm::Value::from(id)],
    );
    let row =
        <sea_orm::DatabaseConnection as sea_orm::ConnectionTrait>::query_one_raw(&ctx.db, stmt)
            .await
            .expect("querying the stored post");
    let stored = row
        .expect("the post row must exist")
        .try_get::<Option<chrono::DateTime<chrono::FixedOffset>>>("", "due_date")
        .expect("reading the due_date column");
    assert!(
        stored.is_none(),
        "the stored post's due date must be NULL, got: {stored:?}"
    );
}

#[when("Anna lists posts")]
async fn anna_lists_posts(world: &mut PostsWorld) {
    let res = world.server().get("/api/posts").await;
    world.record_response(res);
}

#[then("every post in the list has a due date field")]
fn every_listed_post_has_due_date_field(world: &mut PostsWorld) {
    let items = world.list_items();
    assert!(
        !items.is_empty(),
        "the list must contain at least the seeded post"
    );
    for post in &items {
        assert!(
            post.get("due_date").is_some(),
            "a post representation must carry a due_date field (RD-2), got: {post}"
        );
    }
}

#[when("Anna fetches that post")]
async fn anna_fetches_that_post(world: &mut PostsWorld) {
    let (id, _) = world.last_post.clone().expect("no post to fetch");
    let res = world.server().get(&format!("/api/posts/{id}")).await;
    world.record_response(res);
}

#[then("the post in the response has a due date field")]
fn fetched_post_has_due_date_field(world: &mut PostsWorld) {
    let body = world.last_body.as_ref().expect("response must be JSON");
    assert!(
        body.get("due_date").is_some(),
        "a post representation must carry a due_date field (RD-2), got: {body}"
    );
}

#[then(expr = "no post titled {string} exists")]
async fn no_post_titled_exists(world: &mut PostsWorld, title: String) {
    let res = world.server().get("/api/posts?page_size=1000").await;
    world.record_response(res);
    assert_eq!(world.last_status, Some(200), "listing posts must succeed");
    assert!(
        !world
            .list_items()
            .iter()
            .any(|p| p["title"].as_str() == Some(title.as_str())),
        "post {title:?} should not exist"
    );
}

#[when(regex = r#"Anna updates that post with a due date (.+)"#)]
async fn update_that_post_with_due_date(world: &mut PostsWorld, due_date: String) {
    let (id, title) = world.last_post.clone().expect("no post to update");
    let res = world.server().get(&format!("/api/posts/{id}")).await;
    world.record_response(res);
    assert_eq!(
        world.last_status,
        Some(200),
        "the post must exist before the update"
    );
    world.due_date_snapshot = world
        .last_body
        .as_ref()
        .and_then(|b| b.get("due_date"))
        .cloned()
        .or(Some(Value::Null));

    let res = world
        .server()
        .put(&format!("/api/posts/{id}"))
        .json(&json!({ "title": title, "due_date": due_date }))
        .await;
    world.record_response(res);
}

#[then("the stored post's due date is unchanged")]
async fn stored_post_due_date_unchanged(world: &mut PostsWorld) {
    let (id, _) = world.last_post.clone().expect("no post to inspect");
    let res = world.server().get(&format!("/api/posts/{id}")).await;
    world.record_response(res);
    assert_eq!(
        world.last_status,
        Some(200),
        "fetching the post must succeed"
    );
    let after = world
        .last_body
        .as_ref()
        .and_then(|b| b.get("due_date"))
        .cloned()
        .unwrap_or(Value::Null);
    assert_eq!(
        after,
        world.due_date_snapshot.clone().unwrap_or(Value::Null),
        "the stored due date changed"
    );
}
