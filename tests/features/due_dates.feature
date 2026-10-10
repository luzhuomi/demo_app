Feature: Optional due dates on posts and overdue filtering
  # Spec: specs/due-dates.md (RD-1..RD-15, answers Q1..Q8)
  #
  # Resolved decisions baked into these scenarios:
  #   Q1  "overdue" means due_date strictly before now
  #   Q2  "now" is the viewer's local time, communicated via the `timezone`
  #       query parameter (IANA name, default UTC, 400 for unknown zone — Q9)
  #   Q3  only published posts can be overdue
  #   Q4  ?overdue=false, and any value other than "true", is a 400
  #   Q5  omitting due_date in an update clears it (same as explicit null)
  #   Q6  every post representation carries a computed `overdue` bool;
  #       unpublished past-due posts are not overdue (Q10)
  #   Q10 also adds a computed is_published bool to the representation
  #       (API-only, not a database column — Q11)
  #   Q7  lists are ordered by due_date asc by default (nulls last);
  #       sort_by=id restores id order; invalid sort_by is a 400
  #   Q8  past due_date values are rejected on create/update (400)
  #       (evaluated against the UTC server clock — see spec question Q9)
  #
  # Seeding note: because Q8 rejects past due dates on write, every
  # "due in the past" precondition is seeded directly in the database,
  # bypassing the API.

  Background:
    Given the posts API is running

  @RD-1
  Scenario: A post created without a due date has a null due date
    When Anna creates a post titled "Draft" without a due date
    Then the response status is 201
    And the created post has no due date
    And the stored post has no due date

  @RD-2
  Scenario: Post representations include due_date on list and detail
    Given the posts table contains a published post due in 2 days
    When Anna lists posts
    Then the response status is 200
    And every post in the list has a due date field
    When Anna fetches that post
    Then the response status is 200
    And the post in the response has a due date field

  @RD-3
  Scenario: A valid future due date is persisted and echoed on create and update
    When Anna creates a post titled "Launch" with a due date 2 days from now
    Then the response status is 201
    And the created post has a due date 2 days from now
    When Anna updates that post with a due date 5 days from now
    Then the response status is 200
    And the post in the response has a due date 5 days from now
    When Anna fetches that post
    Then the post in the response has a due date 5 days from now

  @RD-4
  Scenario: A null or omitted due date on update clears the due date
    When Anna creates a post titled "Launch" with a due date 2 days from now
    Then the response status is 201
    When Anna updates that post setting the due date to null
    Then the response status is 200
    And the post in the response has no due date
    When Anna creates a post titled "Relaunch" with a due date 2 days from now
    And Anna updates that post omitting the due date
    Then the post in the response has no due date

  @RD-5
  Scenario: The overdue filter returns only published posts due in the past
    Given the posts table contains a published post due 2 days ago
    And the posts table contains an unpublished post due 2 days ago
    And the posts table contains a published post due in 2 days
    And the posts table contains a published post with no due date
    When Anna lists posts with the overdue filter
    Then the response status is 200
    And the list contains exactly the published post due 2 days ago

  @RD-6
  Scenario: Listing without the overdue filter returns every post
    Given the posts table contains a published post due 2 days ago
    And the posts table contains an unpublished post due 2 days ago
    And the posts table contains a published post due in 2 days
    And the posts table contains a published post with no due date
    When Anna lists posts
    Then the response status is 200
    And the list contains 4 posts

  @RD-7
  Scenario: The overdue filter applies before pagination
    Given the posts table contains a published post due 5 days ago
    And the posts table contains a published post due 2 days ago
    And the posts table contains a published post due 1 day ago
    And the posts table contains a published post due in 2 days
    When Anna lists posts with the overdue filter and page size 2
    Then the response status is 200
    And the list contains 2 posts
    And the total item count is 3

  @RD-8
  Scenario: Posts without a due date are never overdue
    Given the posts table contains a published post with no due date
    And the posts table contains an unpublished post with no due date
    When Anna lists posts with the overdue filter
    Then the response status is 200
    And the list is empty

  @RD-9
  Scenario: Overdue is evaluated in the viewer's timezone
    Given the posts table contains a published post due 5 hours ago
    When Anna lists posts with the overdue filter
    Then the list contains the post due 5 hours ago
    When Anna lists posts with the overdue filter in timezone Pacific/Honolulu
    Then the list does not contain the post due 5 hours ago

  @Q9
  Scenario: An unknown timezone is rejected
    When Anna lists posts with the overdue filter in timezone Mars/OlympusMons
    Then the response status is 400

  @RD-10
  Scenario: An invalid due date is rejected and nothing is stored
    When Anna creates a post titled "Broken" with a due date not-a-date
    Then the response status is 400
    And no post titled "Broken" exists
    When Anna creates a post titled "Broken" with a due date 2024-02-30T09:00:00Z
    Then the response status is 400
    And no post titled "Broken" exists
    When Anna creates a post titled "Broken" with a due date 10000-01-01T00:00:00Z
    Then the response status is 400
    And no post titled "Broken" exists

  @Q8
  Scenario: A past due date is rejected on create and update
    Given the posts table contains a published post due in 2 days
    When Anna creates a post titled "Late" with a due date 2020-01-01T00:00:00Z
    Then the response status is 400
    And no post titled "Late" exists
    When Anna updates that post with a due date 2020-01-01T00:00:00Z
    Then the response status is 400
    And the stored post's due date is unchanged

  @RD-11
  Scenario: An invalid JSON body is rejected and nothing is stored
    When Anna sends "not json" as the create body
    Then the response status is 400
    And no post was created

  @RD-12
  Scenario: A non-boolean overdue parameter is rejected
    When Anna lists posts with overdue set to banana
    Then the response status is 400

  @Q4
  Scenario: overdue=false is rejected as an invalid value
    When Anna lists posts with overdue set to false
    Then the response status is 400

  @RD-13
  Scenario: An overdue query matching nothing returns an empty page
    Given the posts table contains a published post due in 2 days
    When Anna lists posts with the overdue filter
    Then the response status is 200
    And the list is empty
    And the total item count is 0

  @RD-14
  Scenario: Non-existent post ids return 404 on get, update and delete
    When Anna fetches post 999999
    Then the response status is 404
    When Anna updates post 999999 with the title "Ghost"
    Then the response status is 404
    When Anna deletes post 999999
    Then the response status is 404

  @RD-15
  Scenario: Requests that use no new parameter keep working as before
    Given the posts table contains a published post due in 2 days
    When Anna lists posts
    Then the response status is 200
    And the list response has the pagination fields
    When Anna creates a post titled "Legacy" without a due date
    Then the response status is 201
    And the created post has no due date

  @Q6
  Scenario: Post representations carry a computed overdue flag
    Given the posts table contains a published post due 2 days ago
    And the posts table contains an unpublished post due 2 days ago
    And the posts table contains a published post due in 2 days
    And the posts table contains a published post with no due date
    When Anna lists posts
    Then the response status is 200
    And the published post due 2 days ago has the overdue flag true
    And the published post due in 2 days has the overdue flag false
    And the published post with no due date has the overdue flag false
    And the unpublished post due 2 days ago has the overdue flag false

  @Q10
  Scenario: Post representations carry a publication bool
    Given the posts table contains a published post with no due date
    And the posts table contains an unpublished post with no due date
    And the posts table contains a post with null published
    When Anna lists posts
    Then the response status is 200
    And the published post has is_published true
    And the unpublished post has is_published false
    And the post with null published has is_published false

  @Q7
  Scenario: Lists are ordered by due date by default, posts without a due date last
    Given the posts table contains a published post due in 5 days
    And the posts table contains a published post due in 2 days
    And the posts table contains a published post with no due date
    When Anna lists posts
    Then the list is in due date order with posts without a due date last

  @Q7
  Scenario: sort_by=id restores id ordering
    Given the posts table contains a published post due in 5 days
    And the posts table contains a published post due in 2 days
    And the posts table contains a published post with no due date
    When Anna lists posts with sort_by set to id
    Then the list is in id order

  @Q7
  Scenario: An invalid sort_by value is rejected
    When Anna lists posts with sort_by set to banana
    Then the response status is 400

  @Q7
  Scenario: The overdue list is ordered most overdue first
    Given the posts table contains a published post due 1 day ago
    And the posts table contains a published post due 5 days ago
    And the posts table contains a published post due 2 days ago
    When Anna lists posts with the overdue filter
    Then the list is most overdue first
