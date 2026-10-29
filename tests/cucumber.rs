mod steps;

use axum_test::TestServer;
use cucumber::World;
use serde_json::Value;

#[derive(Default, cucumber::World)]
pub struct PostsWorld {
    /// Real application server, booted by the `the posts API is running` step.
    pub server: Option<TestServer>,
    /// App context of that boot, for steps that read the database directly
    /// (e.g. asserting stored columns).
    pub ctx: Option<loco_rs::prelude::AppContext>,
    /// Status code of the last HTTP response recorded in this scenario.
    pub last_status: Option<u16>,
    /// JSON body of the last HTTP response (None when the body is not JSON).
    pub last_body: Option<Value>,
    /// The most recently created/seeded post: `(id, title)` — "that post".
    pub last_post: Option<(i64, String)>,
    /// The `due_date` JSON value of "that post", read just before an update.
    pub due_date_snapshot: Option<Value>,
}

// `AppContext` is not `Debug`, so the derive can't be used; the World macro
// only needs `Debug` to exist.
impl std::fmt::Debug for PostsWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("PostsWorld")
            .field("server", &self.server.is_some())
            .field("ctx", &self.ctx.is_some())
            .field("last_status", &self.last_status)
            .field("last_body", &self.last_body)
            .field("last_post", &self.last_post)
            .field("due_date_snapshot", &self.due_date_snapshot)
            .finish()
    }
}

#[tokio::main]
async fn main() {
    // Phase 1 smoke feature (harness plumbing).
    // PostsWorld::run("tests/features/smoke.feature").await;

    // Steps exist only for the tasks listed here — one task per prompt. The
    // other scenarios in due_dates.feature have no steps yet by design, so
    // the run is filtered by the FR tags those tasks' scenarios carry.
    const STEPPED_TASKS: [&str; 4] = ["RD-1", "RD-2", "RD-3", "Q8"];
    // One concurrent scenario at a time: each boot recreates the test schema
    // (config/test.yaml), so concurrent scenarios would wipe each other's
    // data and race on the seaql_migrations table.
    PostsWorld::cucumber()
        .max_concurrent_scenarios(1)
        .filter_run("tests/features/due_dates.feature", |_, _, scenario| {
            scenario
                .tags
                .iter()
                .any(|tag| STEPPED_TASKS.contains(&tag.as_str()))
        })
        .await;
}
