Feature: Object operations

  Background:
    Given bucket "obj-bucket" exists

  Scenario: Put and get an object
    When I put object "obj-bucket/hello.txt" with content "hello world"
    Then object "obj-bucket/hello.txt" should contain "hello world"

  Scenario: Delete an object
    Given object "obj-bucket/to-delete.txt" contains "temp"
    When I delete object "obj-bucket/to-delete.txt"
    Then object "obj-bucket/to-delete.txt" should not exist

  Scenario: List objects with prefix
    Given object "obj-bucket/a/1.txt" contains "one"
    And object "obj-bucket/a/2.txt" contains "two"
    And object "obj-bucket/b/1.txt" contains "three"
    When I list objects in "obj-bucket" with prefix "a/"
    Then the listed keys should be "a/1.txt,a/2.txt"

  Scenario: Delete a non-existent object succeeds
    When I delete object "obj-bucket/ghost.txt"
    Then object "obj-bucket/ghost.txt" should not exist

  Scenario: Get non-existent object fails
    When I try to get object "obj-bucket/nope.txt"
    Then the operation should fail

  Scenario: HEAD object returns metadata
    When I put object "obj-bucket/head-test.txt" with content "headme"
    When I head object "obj-bucket/head-test.txt"
    Then the head response content length should be "6"
    And the head response should have an etag

  Scenario: Put and get with content type
    When I upload "obj-bucket/typed.json" with body "{}" and content type "application/json"
    Then object "obj-bucket/typed.json" should have content type "application/json"

  Scenario: Put and get with custom metadata
    When I upload "obj-bucket/meta.txt" with body "data" and metadata "color=blue,env=test"
    Then object "obj-bucket/meta.txt" should have metadata "color" with value "blue"
    And object "obj-bucket/meta.txt" should have metadata "env" with value "test"

  Scenario: Last-Modified header uses RFC 7231 format
    When I put object "obj-bucket/rfc-test.txt" with content "ts"
    Then the last modified header for "obj-bucket/rfc-test.txt" should be RFC 7231 format
