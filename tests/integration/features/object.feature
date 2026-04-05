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

  Scenario: Get non-existent object fails
    When I try to get object "obj-bucket/nope.txt"
    Then the operation should fail
