Feature: aws-chunked content encoding

  Background:
    Given bucket "chunked-bucket" exists

  Scenario: PutObject with aws-chunked encoding stores decoded content
    When I put an aws-chunked object "chunked-bucket/test.json" with content '{"status":"ok"}'
    Then the aws-chunked object "chunked-bucket/test.json" should contain '{"status":"ok"}'

  Scenario: Large aws-chunked upload with multiple chunks
    When I put a large aws-chunked object "chunked-bucket/big.bin" of 32768 bytes
    Then object "chunked-bucket/big.bin" should have content length 32768
