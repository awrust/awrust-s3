Feature: Object tagging

  Background:
    Given bucket "tag-bucket" exists

  Scenario: Set and get tags
    Given object "tag-bucket/tagged.txt" contains "data"
    When I set tags on "tag-bucket/tagged.txt" with "env=prod,team=data"
    Then object "tag-bucket/tagged.txt" should have tag "env" with value "prod"
    And object "tag-bucket/tagged.txt" should have tag "team" with value "data"

  Scenario: Get tags returns empty when none set
    Given object "tag-bucket/no-tags.txt" contains "data"
    Then object "tag-bucket/no-tags.txt" should have no tags

  Scenario: Delete tags
    Given object "tag-bucket/del-tags.txt" contains "data"
    When I set tags on "tag-bucket/del-tags.txt" with "env=prod"
    And I delete tags on "tag-bucket/del-tags.txt"
    Then object "tag-bucket/del-tags.txt" should have no tags

  Scenario: Put object clears existing tags
    Given object "tag-bucket/overwrite.txt" contains "v1"
    When I set tags on "tag-bucket/overwrite.txt" with "env=prod"
    And I put object "tag-bucket/overwrite.txt" with content "v2"
    Then object "tag-bucket/overwrite.txt" should have no tags

  Scenario: Tagging a non-existent object fails
    When I try to get tags on "tag-bucket/nope.txt"
    Then the tagging operation should fail
