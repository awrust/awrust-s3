Feature: Virtual-host style addressing

  Scenario: Put and get object via virtual-host
    Given bucket "vhost-bucket" exists
    When I put object "hello.txt" with body "hello world" via virtual-host to "vhost-bucket"
    Then I can get object "hello.txt" via virtual-host from "vhost-bucket" with body "hello world"

  Scenario: Create and head bucket via virtual-host
    When I create bucket "vhost-create" via virtual-host
    Then I can head bucket "vhost-create" via virtual-host

  Scenario: List objects via virtual-host
    Given bucket "vhost-list" exists
    And object "vhost-list/a.txt" contains "aaa"
    And object "vhost-list/b.txt" contains "bbb"
    When I list objects via virtual-host from "vhost-list"
    Then the virtual-host listing should contain "a.txt"
    And the virtual-host listing should contain "b.txt"

  Scenario: Delete object via virtual-host
    Given bucket "vhost-del" exists
    And object "vhost-del/doomed.txt" contains "bye"
    When I vhost-delete object "doomed.txt" from "vhost-del"
    Then getting object "doomed.txt" via virtual-host from "vhost-del" should fail

  Scenario: Path-style still works alongside virtual-host
    Given bucket "dual-bucket" exists
    And object "dual-bucket/path-obj.txt" contains "via path"
    When I get object "dual-bucket/path-obj.txt" via path-style
    Then the path-style response body should be "via path"

  Scenario: Cross-style access
    Given bucket "cross-bucket" exists
    When I put object "cross.txt" with body "cross data" via virtual-host to "cross-bucket"
    Then object "cross-bucket/cross.txt" should contain "cross data"
