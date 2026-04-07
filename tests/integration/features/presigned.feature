Feature: Presigned URL support

  Background:
    Given bucket "presigned-bucket" exists

  Scenario: PUT object via presigned URL
    When I upload "presigned-bucket/put-test.txt" via presigned URL with content "presigned put"
    Then object "presigned-bucket/put-test.txt" should contain "presigned put"

  Scenario: GET object via presigned URL
    Given object "presigned-bucket/get-test.txt" contains "presigned get"
    When I download "presigned-bucket/get-test.txt" via presigned URL
    Then the presigned response should contain "presigned get"
