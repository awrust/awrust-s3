Feature: Batch delete objects

  Background:
    Given bucket "batch-bucket" exists

  Scenario: Delete multiple existing objects
    Given object "batch-bucket/a.txt" contains "alpha"
    And object "batch-bucket/b.txt" contains "bravo"
    And object "batch-bucket/c.txt" contains "charlie"
    When I batch delete keys "a.txt,b.txt,c.txt" from "batch-bucket"
    Then the batch delete response should have 3 deleted keys
    And the batch delete response should have 0 errors
    And object "batch-bucket/a.txt" should not exist
    And object "batch-bucket/b.txt" should not exist
    And object "batch-bucket/c.txt" should not exist

  Scenario: Delete mix of existing and non-existing keys
    Given object "batch-bucket/real.txt" contains "data"
    When I batch delete keys "real.txt,ghost.txt" from "batch-bucket"
    Then the batch delete response should have 2 deleted keys
    And the batch delete response should have 0 errors
    And object "batch-bucket/real.txt" should not exist

  Scenario: Delete non-existing keys from empty bucket
    When I batch delete keys "nope1.txt,nope2.txt" from "batch-bucket"
    Then the batch delete response should have 2 deleted keys
    And the batch delete response should have 0 errors

  Scenario: Quiet mode suppresses deleted entries
    Given object "batch-bucket/q.txt" contains "quiet"
    When I batch delete keys "q.txt" from "batch-bucket" in quiet mode
    Then the batch delete response should have 0 deleted keys
    And the batch delete response should have 0 errors
    And object "batch-bucket/q.txt" should not exist
