Feature: Range requests

  Background:
    Given bucket "range-bucket" exists

  Scenario: Download first N bytes
    Given object "range-bucket/data.bin" has 1000 bytes of known content
    When I get bytes 0 to 99 of "range-bucket/data.bin"
    Then the response status should be 206
    And the response body should be 100 bytes
    And the response body should match bytes 0 to 99 of the original

  Scenario: Download from offset to end
    Given object "range-bucket/tail.bin" has 500 bytes of known content
    When I get bytes from 400 to end of "range-bucket/tail.bin"
    Then the response status should be 206
    And the response body should be 100 bytes

  Scenario: Download last N bytes
    Given object "range-bucket/suffix.bin" has 500 bytes of known content
    When I get the last 50 bytes of "range-bucket/suffix.bin"
    Then the response status should be 206
    And the response body should be 50 bytes

  Scenario: Full GET includes Accept-Ranges header
    Given object "range-bucket/full.txt" contains "hello"
    When I get object "range-bucket/full.txt"
    Then the response should have header "Accept-Ranges" with value "bytes"
