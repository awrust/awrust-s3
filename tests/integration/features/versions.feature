Feature: ListObjectVersions stub

  Background:
    Given bucket "ver-bucket" exists

  Scenario: List versions returns objects with VersionId null
    Given object "ver-bucket/a.txt" contains "alpha"
    And object "ver-bucket/b.txt" contains "beta"
    When I list object versions in "ver-bucket"
    Then the version list should contain keys "a.txt,b.txt"
    And every version id should be "null"
    And every version should be latest

  Scenario: List versions on empty bucket returns empty list
    Given bucket "ver-empty" exists
    When I list object versions in "ver-empty"
    Then the version list should be empty

  Scenario: List versions with prefix filter
    Given object "ver-bucket/x/1.txt" contains "one"
    And object "ver-bucket/x/2.txt" contains "two"
    And object "ver-bucket/y/1.txt" contains "three"
    And version list prefix is "x/"
    When I list object versions in "ver-bucket"
    Then the version list should contain keys "x/1.txt,x/2.txt"
