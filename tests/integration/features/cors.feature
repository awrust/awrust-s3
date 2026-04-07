Feature: CORS preflight

  Scenario: OPTIONS request returns CORS headers
    When I send an OPTIONS preflight for "PUT" on "/test-bucket/test-key"
    Then the CORS response status should be 200
    And the CORS response should include header "access-control-allow-origin"
    And the CORS response should include header "access-control-allow-methods"
    And the CORS response should include header "access-control-allow-headers"
    And the CORS response body should be empty

  Scenario: Normal GET includes CORS origin header
    When I send a GET request to "/health"
    Then the CORS response status should be 200
    And the CORS response should include header "access-control-allow-origin"
