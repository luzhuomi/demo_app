# Spec: Optional due dates on posts + overdue filter

Status: closed — all questions (Q1–Q11) answered. Feature file:
`tests/features/due_dates.feature`.

Constitution constraints applied:
- Every requirement maps to one Gherkin scenario, written (and seen red) before implementation.
- No breaking changes to existing endpoints.
- One task per prompt; never run past a failing test.
- Stack: Loco, SeaORM, Axum only.

## Scope

Add a nullable `due_date` to `posts`, exposed on create/update/read, plus an
`overdue` filter on `GET /api/posts`.

## Requirements (EARS)

### Data model

- **RD-1** [Ubiquitous] The system shall store an optional `due_date`
  (nullable `timestamptz`) on the `posts` table.
- **RD-2** [Ubiquitous] Every post representation (`PostDto` on list and
  detail) shall include `due_date` as an RFC 3339 timestamp or `null`.
  (Additive JSON field — non-breaking per constitution.)

### Create / update

- **RD-3** [When] When a client creates or updates a post with a valid
  RFC 3339 `due_date`, the system shall persist it and echo it in the response.
- **RD-4** [When] When a client creates or updates a post with `due_date`
  explicitly `null`, the system shall store `null` (clears on update, matching
  the existing `published` convention).

### Overdue filter

- **RD-5** [Where] Where the list endpoint receives `?overdue=true`, the system
  shall return only posts whose `due_date` is non-null and in the past, and no
  others.
- **RD-6** [When] When the list endpoint is called without `overdue`, the
  system shall return posts exactly as today (no filtering by due date).
- **RD-7** [When] When `overdue=true` is combined with pagination, the system
  shall apply the filter before paginating and report correct `total` counts.
- **RD-8** [While] While a post has no `due_date` (null), the system shall
  never include it in an `overdue=true` result, regardless of its `published`
  state.
- **RD-9** [Ubiquitous] The system shall evaluate "overdue" using the current
  UTC time at request time.

### Sad paths (EARS "unwanted behavior" pattern)

- **RD-10** [If/Then] If the request body contains a `due_date` that is not
  valid RFC 3339 (malformed, non-existent date like `2024-02-30`, or out of
  range), then the system shall return 400 with the `ApiError` envelope and
  leave the post unmodified.
- **RD-11** [If/Then] If the request body is not valid JSON, then the system
  shall return 400 with the `ApiError` envelope (existing behavior preserved).
- **RD-12** [If/Then] If `overdue` is present but not parseable as a boolean
  (e.g. `?overdue=banana`), then the system shall return 400 with the `ApiError`
  envelope.
- **RD-13** [If/Then] If an `overdue=true` query matches no posts, then the
  system shall return 200 with an empty `Page` (not 404).
- **RD-14** [If/Then] If a detail/update/delete targets a non-existent post id,
  then the system shall return 404 with the `ApiError` envelope (existing
  behavior, regression-covered).

### Cross-cutting

- **RD-15** [Ubiquitous] For any request that omits `due_date` and `overdue`,
  existing routes, status codes, and response shapes (minus the added field)
  shall be unchanged.

## Open questions

Answer each in the `Answer:` line below. Requirements that depend on an answer
are marked with the question number.

1. **Boundary** *(affects RD-5, RD-9)*: Is a post due exactly at the evaluation
   instant overdue (`<= now`) or not (`< now`)?
   - Answer: < now
2. **Timezone** *(affects RD-9)*: Is "now" always UTC, or should it honor a
   client/viewer timezone?
   - Answer: client/viewer timezone
3. **Published scope** *(affects RD-5, RD-8)*: Should `overdue=true` consider
   all posts, or only `published` ones?
   - Answer: only `published` ones.
4. **`overdue=false` semantics** *(affects RD-12)*: Does it mean "only
   non-overdue posts" (inverse filter), "no filter" (same as omitted), or is it
   a 400 invalid value?
   - Answer: return 400 invalid value.
5. **Update omission semantics** *(affects RD-4)*: Current `UpdatePost` treats
   a missing field as "set null" (how `published` works today). Should omitted
   `due_date` in an update clear it (consistent) or be a no-op (requires a
   sentinel, diverging from convention)?
   - Answer: an update clear it.
6. **Derived field**: Should responses also carry a computed `overdue: bool`,
   or is filtering the only concern?
   - Answer: carry a computed `overdue: bool`
7. **Ordering** *(affects RD-7)*: Keep `id` asc under the filter, or sort
   most-overdue-first (`due_date` asc)?
   - Answer: we should have two options, a `sort_by` flag? which is default to sort by `due_date`.
8. **Validation on write**: Should the API reject future `due_date`s (or past
   ones) on create/update, or accept any instant?
   - Answer: we should reject past `due_date`s.

### Notes on resolved answers

- Q2 (viewer timezone) needs a transport mechanism — see Q9 below.
- Q10 confirms the computed-flag predicate (unpublished past-due posts are not
  overdue) and adds a publication bool to the representation; Q11 resolves it
  to a computed `is_published: bool` on the DTO — API response only, no new
  database column.
- Q7 changes the **default list ordering** from `id` asc to `due_date` asc
  (nulls last). RD-15 still holds for routes/status codes/shape; ordering is
  the one deliberate change, and legacy clients can opt back in with
  `sort_by=id`.
- Because of Q8, "due in the past" test fixtures must be seeded in the
  database directly (the API will refuse them).

## Open questions — round 2 (raised by the answers)

9. **Timezone transport** *(raised by Q2)*: How does the client communicate its
   timezone? The feature file provisionally assumes an optional `timezone`
   query parameter with an IANA name (e.g. `?overdue=true&timezone=Pacific/Honolulu`),
   defaulting to UTC when omitted, 400 for an unknown zone, applied to both the
   overdue filter and the computed flag. Q8's write-time validation is
   provisionally assumed to use the UTC server clock (write endpoints have no
   timezone parameter).
   - Answer: `timezone` query parameter with an IANA name, default to UTC, 400 for an unknown zone. 
10. **Computed flag predicate** *(raised by Q6)*: Does the computed `overdue`
    bool on a single post use the same predicate as the filter — published AND
    due in the past — so an unpublished past-due post shows `overdue: false`?
    The feature file provisionally assumes yes.
    - Answer: yes, computed `overdue` bool on a single post, include a new field `published` bool. 
11. **Publication bool: name collision** *(raised by Q10)*: `PostDto` already
    exposes `published: Option<bool>` from the table column, so a *new* field
    cannot also be named `published`. Options:
    (a) add a distinct field, e.g. `is_published: bool` — `true` only when the
    column is `true`, `null` → `false` (additive, non-breaking); the feature
    file provisionally assumes this;
    (b) change the existing field's type from `Option<bool>` to `bool` —
    changes the existing response shape (`null` becomes `false`), which the
    constitution's no-breaking-changes rule argues against.
    Which did you intend, and what should the field be called?
    - Answer: (a), this is a computed field showing up only in the API response, not in the database record.

## Task breakdown (proposed, after answers are in)

One task per prompt, each starting from a failing Gherkin scenario:

1. Migration: add nullable `due_date` to `posts` (+ codegen).
2. DTOs: `due_date` on `PostDto` / `CreatePost` / `UpdatePost`; computed
   `overdue` bool and `is_published` bool (API-only, no new column) on `PostDto`.
3. Create/update accept + persist `due_date` (incl. RD-10, RD-11, Q8 past-date
   rejection).
4. List: `?overdue` filter, `timezone` param, `sort_by` (incl. RD-12, RD-13,
   Q4, Q7, Q9) + pagination interaction.
5. Regression pass over RD-14 / RD-15.
