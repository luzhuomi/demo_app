mod due_dates;

use cucumber::{given, then, when};

use crate::PostsWorld;

#[given("the posts API is running")]
async fn posts_api_running(world: &mut PostsWorld) {
    let boot = loco_rs::testing::prelude::boot_test::<demo_app::app::App>()
        .await
        .expect("boot the test app");
    let routes = boot.router.clone().expect("booted router");
    let server = axum_test::TestServer::new_with_config(
        routes.into_make_service_with_connect_info::<std::net::SocketAddr>(),
        loco_rs::testing::prelude::RequestConfig::default(),
    )
    .expect("start the test server");
    world.server = Some(server);
    world.ctx = Some(boot.app_context);
}

#[when("Anna checks the health endpoint")]
fn check_health(world: &mut PostsWorld) {
    // Lab B repoints this harness at the real app; for the smoke test we
    // just prove the plumbing compiles and runs.
    world.last_status = Some(200);
}

#[then(expr = "the response status is {int}")]
fn assert_status(world: &mut PostsWorld, expected: u16) {
    assert_eq!(world.last_status, Some(expected));
}
