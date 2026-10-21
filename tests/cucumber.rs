mod steps;

use axum_test::TestServer;
use cucumber::World;
use serde_json::Value;

#[derive(Debug, Default, cucumber::World)]
pub struct PostsWorld {
    /// Real application server, booted by the `the posts API is running` step.
    pub server: Option<TestServer>,
    /// Status code of the last HTTP response recorded in this scenario.
    pub last_status: Option<u16>,
    /// JSON body of the last HTTP response (None when the body is not JSON).
    pub last_body: Option<Value>,
    /// The most recently created/seeded post: `(id, title)` — "that post".
    pub last_post: Option<(i64, String)>,
    /// The `due_date` JSON value of "that post", read just before an update.
    pub due_date_snapshot: Option<Value>,
}

#[tokio::main]
async fn main() {
    // Phase 1 smoke feature (harness plumbing).
    // PostsWorld::run("tests/features/smoke.feature").await;

    // BDD red for T-12 (specs/due_dates/tasks.md): step definitions exist
    // only for this scenario — one task per prompt. The other scenarios in
    // due_dates.feature have no steps yet by design, so the run is filtered
    // to the T-12 scenario.
    PostsWorld::filter_run("tests/features/due_dates.feature", |_, _, scenario| {
        scenario.name == "A past due date is rejected on create and update"
    })
    .await;
}
