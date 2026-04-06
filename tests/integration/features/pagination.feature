Feature: ListObjectsV2 pagination

  Background:
    Given bucket "page-bucket" exists

  Scenario: Paginate object listing
    Given 10 objects exist in "page-bucket" with prefix "item/"
    When I list objects in "page-bucket" with prefix "item/" and max keys 3
    Then 3 objects should be returned
    And the response should be truncated
    When I list the next page
    Then 3 objects should be returned
    And the response should be truncated
    When I list the next page
    Then 3 objects should be returned
    And the response should be truncated
    When I list the next page
    Then 1 objects should be returned
    And the response should not be truncated

  Scenario: List without pagination returns all
    Given 5 objects exist in "page-bucket" with prefix "all/"
    When I list objects in "page-bucket" with prefix "all/" and max keys 1000
    Then 5 objects should be returned
    And the response should not be truncated
