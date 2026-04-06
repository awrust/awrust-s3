Feature: Multipart upload

  Background:
    Given bucket "mp-bucket" exists

  Scenario: Multipart upload and download
    When I upload "mp-bucket/big.bin" as multipart with 3 parts of 5MB each
    Then object "mp-bucket/big.bin" should have size 15728640
    And I can download "mp-bucket/big.bin" and it matches the uploaded content

  Scenario: Abort multipart upload
    When I initiate a multipart upload for "mp-bucket/aborted.bin"
    And I abort the multipart upload
    Then object "mp-bucket/aborted.bin" should not exist

  Scenario: AWS CLI copies a large file via multipart
    When I copy a 10MB file to "mp-bucket/large.bin" using aws s3 cp
    Then object "mp-bucket/large.bin" should have size 10485760
