Feature: List multipart uploads
  As an S3 client
  I want to list in-progress multipart uploads
  So that I can manage or clean them up

  Scenario: List active multipart uploads
    Given bucket "lu-bucket" exists
    When I initiate a multipart upload for "lu-bucket/photos/cat.jpg"
    And I initiate a multipart upload for "lu-bucket/docs/readme.md"
    Then listing multipart uploads in "lu-bucket" returns 2 uploads
    And the upload keys include "photos/cat.jpg" and "docs/readme.md"

  Scenario: List uploads filtered by prefix
    Given bucket "lu-prefix" exists
    When I initiate a multipart upload for "lu-prefix/photos/cat.jpg"
    And I initiate a multipart upload for "lu-prefix/docs/readme.md"
    Then listing multipart uploads in "lu-prefix" with prefix "photos/" returns 1 upload
    And the upload key is "photos/cat.jpg"
