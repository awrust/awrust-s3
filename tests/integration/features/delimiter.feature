Feature: ListObjectsV2 delimiter support

  Background:
    Given bucket "delim-bucket" exists

  Scenario: Delimiter groups nested keys into common prefixes
    Given object "delim-bucket/a.txt" contains "data"
    And object "delim-bucket/dir/b.txt" contains "data"
    And object "delim-bucket/dir/c.txt" contains "data"
    And object "delim-bucket/dir/sub/d.txt" contains "data"
    When I list objects in "delim-bucket" using delimiter "/"
    Then the content keys should be "a.txt"
    And the common prefix list should be "dir/"

  Scenario: Prefix with delimiter returns correct split
    Given object "delim-bucket/photos/a.jpg" contains "data"
    And object "delim-bucket/photos/vacation/b.jpg" contains "data"
    And object "delim-bucket/photos/vacation/c.jpg" contains "data"
    When I list objects in "delim-bucket" using prefix "photos/" and delimiter "/"
    Then the content keys should be "photos/a.jpg"
    And the common prefix list should be "photos/vacation/"

  Scenario: No delimiter returns all keys as contents
    Given object "delim-bucket/x/a.txt" contains "data"
    And object "delim-bucket/x/b.txt" contains "data"
    When I list objects in "delim-bucket" using prefix "x/" without delimiter
    Then the content keys should be "x/a.txt,x/b.txt"
    And the common prefix list should be empty
