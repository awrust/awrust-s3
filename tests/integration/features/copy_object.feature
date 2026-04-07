Feature: Copy object

  Background:
    Given bucket "copy-src" exists
    And bucket "copy-dst" exists

  Scenario: Copy object within same bucket
    Given object "copy-src/original.txt" contains "hello"
    When I copy object "copy-src/original.txt" to "copy-src/duplicate.txt"
    Then object "copy-src/duplicate.txt" should contain "hello"

  Scenario: Copy object across buckets
    Given object "copy-src/cross.txt" contains "cross-bucket"
    When I copy object "copy-src/cross.txt" to "copy-dst/cross.txt"
    Then object "copy-dst/cross.txt" should contain "cross-bucket"

  Scenario: Copy preserves metadata
    When I upload "copy-src/meta.txt" with body "data" and metadata "color=red"
    And I copy object "copy-src/meta.txt" to "copy-dst/meta.txt"
    Then object "copy-dst/meta.txt" should have metadata "color" with value "red"

  Scenario: Copy preserves content type
    When I upload "copy-src/typed.json" with body "{}" and content type "application/json"
    And I copy object "copy-src/typed.json" to "copy-dst/typed.json"
    Then object "copy-dst/typed.json" should have content type "application/json"

  Scenario: Copy from non-existent source fails
    When I try to copy object "copy-src/nope.txt" to "copy-dst/nope.txt"
    Then the operation should fail
