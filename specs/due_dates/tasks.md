# due_dates — task list

One entry per Gherkin scenario in `tests/features/due_dates.feature`.
FR source: `specs/due-dates.md` (RD-1…RD-15; Qx = resolved open-question
decisions, treated as requirements).

Execution order: prerequisites first, then T-01…T-24 in order.
**One task per prompt; never run past a failing test.** (constitution)

Each entry follows the constitution's loop, in this order:

1. **BDD red** — write the scenario's new step definitions in
   `tests/steps/` wired to real behavior (real HTTP via `TestServer`,
   real DB), run Cucumber, and see the scenario fail.
2. **TDD red** — write a failing unit/integration test for the behavior
   (tests in `tests/models/` or `tests/requests/`).
3. **TDD green** — minimal implementation in `src/` until that test passes.
   `cargo fmt --all && cargo clippy --all-targets -- -D warnings && cargo test`.
4. **BDD green** — re-run Cucumber; the scenario passes.

## Prerequisites (one task each, before T-01)

### P-01 · Point the Cucumber harness at the real app
- FR: constitution (BDD harness)
- BDD red: `tests/cucumber.rs` runs only `smoke.feature`; `PostsWorld` is a
  stub. Extend `PostsWorld` to hold a `TestServer` + `AppContext` (real HTTP,
  real DB, posts truncated between scenarios) and run
  `due_dates.feature` alongside `smoke.feature`.
- TDD red: n/a (harness task) — prove it with a failing step: add one
  `due_dates.feature` step definition that makes a real request and assert it
  fails because the feature is unimplemented.
- TDD green: wire `TestServer`/`AppContext` into the world; keep the smoke
  scenario green.
- BDD green: `cargo test --test cucumber` runs both features; smoke passes.

### P-02 · Shared step definitions and world state
- FR: infrastructure for T-01…T-24
- Step definitions to land here (referenced by the entries below):
  - seeding: `the posts table contains a {published|unpublished} post due
    {int} {days|hours} {ago|in}` / `… with no due date` / `a post with null
    published` — insert directly via SeaORM, bypassing the API (Q8 makes the
    API refuse past due dates)
  - actions: `Anna creates/lists/fetches/updates/deletes …` (exact step text
    per scenario, below)
  - assertions on status, list contents, ordering, DTO fields, DB rows
- BDD red: steps exist but the scenarios they serve fail.
- TDD red/green: n/a (steps only; behavior tested by the scenario tasks).
- BDD green: no undefined steps remain for the first scenario in flight.

---

## Scenario tasks

### T-01 · A post created without a due date has a null due date
- FR: RD-1 · `due_dates.feature:27-32`
- BDD red: new steps `Anna creates a post titled {string} without a due date`,
  `the created post has no due date`, `the stored post has no due date` →
  scenario fails (no `due_date` in the DTO; column absent).
- TDD red: `tests/requests` — POST `/api/posts` without `due_date`; expect 201
  and the DB row's `due_date` to be NULL (fails: column does not exist).
- TDD green: migration adding nullable `due_date` to `posts`;
  `cargo loco db migrate && cargo loco db entities`; `due_date:
  Option<DateTimeWithTimeZone>` on `PostDto` + `From<Model>` mapping; create
  handler leaves it unset.
- BDD green: scenario passes.

### T-02 · Post representations include due_date on list and detail
- FR: RD-2 · `due_dates.feature:34-42`
- BDD red: new steps `the posts table contains a published post due in 2 days`,
  `Anna lists posts`, `every post in the list has a due date field`,
  `Anna fetches that post`, `the post in the response has a due date field` →
  scenario fails until list/detail carry the field.
- TDD red: `tests/requests` — GET list and GET detail; both JSON bodies
  contain the `due_date` key.
- TDD green: `PostDto` already carries it from T-01; ensure list and detail
  both serialize via `PostDto`.
- BDD green: scenario passes.

### T-03 · A valid future due date is persisted and echoed on create and update
- FR: RD-3 · `due_dates.feature:44-53`
- BDD red: new steps `Anna creates a post titled {string} with a due date
  {string}` (value: RFC 3339 or relative, e.g. "2 days from now"),
  `the created post has a due date {string}`, `Anna updates that post with a
  due date {string}`, `the post in the response has a due date {string}` →
  fails (body field not accepted).
- TDD red: `tests/requests` — create with `due_date = now + 2d` → 201 and
  echoed; update to `now + 5d` → 200 and echoed; detail returns the updated
  value.
- TDD green: `due_date: Option<…>` on `CreatePost`/`UpdatePost`; controllers
  `Set(params.due_date)`; `PostDto` echoes it.
- BDD green: scenario passes.

### T-04 · A null or omitted due date on update clears the due date
- FR: RD-4 (+ Q5) · `due_dates.feature:55-64`
- BDD red: new steps `Anna updates that post setting the due date to null`,
  `Anna updates that post omitting the due date`, `the post in the response
  has no due date` → fails while omitted/null do not clear.
- TDD red: `tests/requests` — post with a due date; PUT body with
  `due_date: null` → stored NULL; PUT body omitting the key → stored NULL.
- TDD green: `Option` deserialization (missing → `None`) plus
  `Set(params.due_date)` already clears — adjust update handler/DTO so both
  paths store NULL.
- BDD green: scenario passes.

### T-05 · The overdue filter returns only published posts due in the past
- FR: RD-5 (+ Q1, Q3) · `due_dates.feature:66-74`
- BDD red: new steps `the posts table contains an unpublished post due 2 days
  ago`, `the posts table contains a published post due 2 days ago`, `… a
  published post with no due date`, `Anna lists posts with the overdue filter`,
  `the list contains exactly the published post due 2 days ago` → fails
  (filter unknown).
- TDD red: `tests/models/posts.rs` — finder/handler with `overdue=true`
  returns exactly the rows where `published = true AND due_date IS NOT NULL
  AND due_date < now(UTC)`; excludes unpublished-past, future, and
  no-due-date rows.
- TDD green: `overdue` query param on `list` (only `"true"` accepted — see
  T-14); condition built in the model; applied via `query::paginate`.
- BDD green: scenario passes.

### T-06 · Listing without the overdue filter returns every post
- FR: RD-6 · `due_dates.feature:76-84`
- BDD red: new step `the list contains {int} posts` → fails only if the
  filter leaks; expected red is the undefined step.
- TDD red: `tests/requests` — mixed seed, GET list without `overdue` → all 4
  rows present.
- TDD green: none expected (condition applied only when the param is `true`).
  If green immediately: regression lock, keep as guard.
- BDD green: scenario passes.

### T-07 · The overdue filter applies before pagination
- FR: RD-7 · `due_dates.feature:86-95`
- BDD red: new steps `the posts table contains a published post due 5 days
  ago`, `… due 1 day ago`, `Anna lists posts with the overdue filter and page
  size {int}`, `the total item count is {int}` → fails if filter is applied
  after pagination or total is wrong.
- TDD red: `tests/models/posts.rs` — 3 overdue + 1 future; paginate with
  `page_size = 2` under the condition → page has 2 items, `total_items = 3`.
- TDD green: pass the condition into `query::paginate` (count must run on the
  filtered selector).
- BDD green: scenario passes.

### T-08 · Posts without a due date are never overdue
- FR: RD-8 · `due_dates.feature:97-103`
- BDD red: new steps `the posts table contains an unpublished post with no due
  date`, `the list is empty` → expected red: undefined steps; passes once
  T-05's condition includes `due_date IS NOT NULL`.
- TDD red: `tests/models/posts.rs` — seed only no-due-date posts (published
  and unpublished); overdue finder returns zero rows.
- TDD green: none expected if T-05's condition is complete; if green
  immediately: regression lock, keep as guard.
- BDD green: scenario passes.

### T-09 · Overdue is evaluated in the viewer's timezone
- FR: RD-9 (+ Q1, Q2) · `due_dates.feature:105-111`
- BDD red: new steps `the posts table contains a published post due 5 hours
  ago`, `Anna lists posts with the overdue filter in timezone {string}`,
  `the list contains the post due 5 hours ago`, `the list does not contain the
  post due 5 hours ago` → fails (param unknown).
- TDD red: `tests/models/posts.rs` — row due 5h before UTC-now; evaluated in
  UTC → overdue; evaluated in `Pacific/Honolulu` (UTC-10) → not overdue.
- TDD green: `timezone` query param (IANA name, default UTC); convert
  UTC-now to the zone before the `due_date < now` comparison. IANA lookup
  needs a tz database crate (e.g. `chrono-tz`) — a new dependency; call that
  out in the task prompt before adding it.
- BDD green: scenario passes.

### T-10 · An unknown timezone is rejected
- FR: Q9 · `due_dates.feature:113-116`
- BDD red: reuses T-09's timezone step with `Mars/OlympusMons` → fails until
  validation exists.
- TDD red: `tests/models/posts.rs` or `tests/requests` — unknown IANA zone →
  handler-level error (400 `bad_request`), no list returned.
- TDD green: validate the zone during param parsing; reject unknown names with
  `bad_request`.
- BDD green: scenario passes.

### T-11 · An invalid due date is rejected and nothing is stored
- FR: RD-10 · `due_dates.feature:118-128`
- BDD red: reuses `Anna creates a post titled {string} with a due date
  {string}` with `not-a-date`, `2024-02-30T09:00:00Z`,
  `10000-01-01T00:00:00Z`; new step `no post titled {string} exists` → fails
  (invalid values currently deserialize-or-store, or 400 without the
  `ApiError` envelope).
- TDD red: `tests/requests` — each of the three bodies → 400 with the
  `ApiError` shape and zero new rows.
- TDD green: accept `due_date` as a string in the DTO, parse RFC 3339 in the
  model layer, `bad_request` on parse failure (malformed, non-existent date,
  out of range).
- BDD green: scenario passes.

### T-12 · A past due date is rejected on create and update
- FR: Q8 · `due_dates.feature:130-138`
- BDD red: reuses create/update-with-due-date steps with
  `2020-01-01T00:00:00Z`; new step `the stored post's due date is unchanged`
  → fails (past values currently stored).
- TDD red: `tests/models/posts.rs` — model-level write validation: past
  `due_date` (vs UTC now, per Q9) rejected on create and update; row
  unchanged on rejected update.
- TDD green: validation in the model write path (`bad_request` / `ModelError`
  mapped to 400), applied before insert/update.
- BDD green: scenario passes.

### T-13 · An invalid JSON body is rejected and nothing is stored
- FR: RD-11 · `due_dates.feature:140-144`
- BDD red: new steps `Anna sends {string} as the create body`, `no post was
  created` → expected red: undefined steps; behavior likely already 400.
- TDD red: `tests/requests` — POST create with a non-JSON body → 400, zero
  new rows. Regression lock: may pass immediately; keep as guard.
- TDD green: none expected (Axum's `Json` rejection already 400s).
- BDD green: scenario passes.

### T-14 · A non-boolean overdue parameter is rejected
- FR: RD-12 · `due_dates.feature:146-149`
- BDD red: new step `Anna lists posts with overdue set to {string}` with
  `banana` → fails (param unvalidated).
- TDD red: `tests/requests` — `?overdue=banana` → 400 `ApiError`.
- TDD green: `overdue` param parses only `"true"`; any other present value →
  `bad_request`.
- BDD green: scenario passes.

### T-15 · overdue=false is rejected as an invalid value
- FR: Q4 (+ RD-12) · `due_dates.feature:151-154`
- BDD red: reuses T-14's step with `false` → fails until `"false"` is also
  rejected.
- TDD red: `tests/requests` — `?overdue=false` → 400.
- TDD green: covered by T-14's "only `true` accepted" rule; extend the test if
  T-14 allowed `false`.
- BDD green: scenario passes.

### T-16 · An overdue query matching nothing returns an empty page
- FR: RD-13 · `due_dates.feature:156-162`
- BDD red: reuses overdue-filter steps + `the list is empty` + `the total item
  count is 0` → expected red: undefined assertion steps.
- TDD red: `tests/requests` — seed only a future-due post; `?overdue=true` →
  200, `items: []`, `total_items: 0`.
- TDD green: none expected (empty `Page` is the natural result of
  `query::paginate`); regression lock if green immediately.
- BDD green: scenario passes.

### T-17 · Non-existent post ids return 404 on get, update and delete
- FR: RD-14 · `due_dates.feature:164-171`
- BDD red: new steps `Anna fetches post {int}`, `Anna updates post {int} with
  the title {string}`, `Anna deletes post {int}` → expected red: undefined
  steps; 404 behavior already exists.
- TDD red: `tests/requests` — GET/PUT/DELETE `/api/posts/999999` → 404 with
  the `ApiError` envelope. Regression lock: may pass immediately; keep as
  guard.
- TDD green: none expected.
- BDD green: scenario passes.

### T-18 · Requests that use no new parameter keep working as before
- FR: RD-15 (+ Q7 ordering note) · `due_dates.feature:173-181`
- BDD red: new step `the list response has the pagination fields` → expected
  red: undefined step.
- TDD red: `tests/requests` — GET list without new params → 200 with
  `items/page/page_size/total_pages/total_items`; create without `due_date` →
  201 with `due_date: null`.
- TDD green: none expected; if the Q7 default reordering (due_date asc, nulls
  last) changes an existing client's expectations, `sort_by=id` is the opt-out
  — lock that here.
- BDD green: scenario passes.

### T-19 · Post representations carry a computed overdue flag
- FR: Q6 (+ Q1, Q3, Q10 predicate) · `due_dates.feature:183-194`
- BDD red: new steps `the published post due 2 days ago has the overdue flag
  true`, `… due in 2 days … false`, `… with no due date … false`,
  `the unpublished post due 2 days ago has the overdue flag false` → fails
  (field absent).
- TDD red: `tests/models/posts.rs` — DTO mapping computes `overdue` as
  `published == Some(true) && due_date < now(tz)` for all four seeded
  shapes; detail view computes with UTC default.
- TDD green: `overdue: bool` on `PostDto`, computed where the zone is known
  (list handler passes the resolved zone; detail defaults to UTC).
- BDD green: scenario passes.

### T-20 · Post representations carry a publication bool
- FR: Q10, Q11 (+ Q6) · `due_dates.feature:196-205`
- BDD red: new steps `the posts table contains a post with null published`,
  `the published post has is_published true`, `the unpublished post has
  is_published false`, `the post with null published has is_published false`
  → fails (field absent).
- TDD red: `tests/models/posts.rs` — `is_published` is `true` only when the
  column is `Some(true)`; `None` and `Some(false)` → `false`; present on both
  list and detail.
- TDD green: `is_published: bool` on `PostDto` (computed, API-only — no new
  column), derived in the same place as `overdue`.
- BDD green: scenario passes.

### T-21 · Lists are ordered by due date by default, posts without a due date last
- FR: Q7 · `due_dates.feature:207-213`
- BDD red: new steps `the posts table contains a published post due in 5 days`,
  `the list is in due date order with posts without a due date last` → fails
  (current order is `id` asc).
- TDD red: `tests/models/posts.rs` — default list order is `due_date` asc
  with NULLs last for mixed seeds.
- TDD green: default ordering `due_date ASC NULLS LAST` on the list selector
  (deliberate behavior change per the spec note; `sort_by=id` is the opt-out).
- BDD green: scenario passes.

### T-22 · sort_by=id restores id ordering
- FR: Q7 · `due_dates.feature:215-221`
- BDD red: new steps `Anna lists posts with sort_by set to {string}`, `the list
  is in id order` → fails (param unknown).
- TDD red: `tests/models/posts.rs` — `?sort_by=id` → rows in `id` asc for a
  seed whose id order differs from due-date order.
- TDD green: `sort_by` param with values `due_date` (default) and `id`;
  unknown values rejected (T-23).
- BDD green: scenario passes.

### T-23 · An invalid sort_by value is rejected
- FR: Q7 · `due_dates.feature:223-226`
- BDD red: reuses T-22's step with `banana` → fails until validation exists.
- TDD red: `tests/requests` — `?sort_by=banana` → 400 `ApiError`.
- TDD green: `sort_by` parsing accepts only `due_date` | `id`; else
  `bad_request`.
- BDD green: scenario passes.

### T-24 · The overdue list is ordered most overdue first
- FR: Q7 (+ RD-7) · `due_dates.feature:228-234`
- BDD red: new step `the list is most overdue first` (seed order deliberately
  differs from id order) → fails if the filtered list keeps id order.
- TDD red: `tests/models/posts.rs` — `?overdue=true` with seeds due 1d/5d/2d
  ago in id order → result is due_date asc (5d, 2d, 1d).
- TDD green: default `due_date` ordering applies under the filter (no
  NULLs-last concern — filtered rows all have due dates).
- BDD green: scenario passes.

---

## Coverage check

| FR | Scenarios |
|---|---|
| RD-1 | T-01 |
| RD-2 | T-02 |
| RD-3 | T-03 |
| RD-4 (+Q5) | T-04 |
| RD-5 (+Q1, Q3) | T-05 |
| RD-6 | T-06 |
| RD-7 | T-07, T-24 |
| RD-8 | T-08 |
| RD-9 (+Q1, Q2) | T-09 |
| RD-10 | T-11 |
| RD-11 | T-13 |
| RD-12 | T-14, T-15 |
| RD-13 | T-16 |
| RD-14 | T-17 |
| RD-15 | T-18 |
| Q4 | T-15 |
| Q6 (+Q10 predicate) | T-19 |
| Q7 | T-21, T-22, T-23, T-24 |
| Q8 | T-12 |
| Q9 | T-10 |
| Q10, Q11 | T-20 |

24 scenarios, 24 entries; every RD-1…RD-15 covered.
