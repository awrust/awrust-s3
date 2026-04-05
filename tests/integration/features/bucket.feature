Feature: Bucket operations

  Scenario: Create a bucket
    When I create bucket "test-bucket"
    Then bucket "test-bucket" should exist

  Scenario: Delete an empty bucket
    Given bucket "cleanup-bucket" exists
    When I delete bucket "cleanup-bucket"
    Then bucket "cleanup-bucket" should not exist

  Scenario: Delete a non-empty bucket fails
    Given bucket "nonempty-bucket" exists
    And object "nonempty-bucket/file.txt" contains "data"
    When I try to delete bucket "nonempty-bucket"
    Then the operation should fail

  Scenario: Create bucket is idempotent
    Given bucket "idem-bucket" exists
    When I create bucket "idem-bucket"
    Then bucket "idem-bucket" should exist

  Scenario: List all buckets
    Given bucket "alpha-bucket" exists
    And bucket "beta-bucket" exists
    When I list all buckets
    Then the bucket list should contain "alpha-bucket"
    And the bucket list should contain "beta-bucket"
