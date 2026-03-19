Feature: MCP Protocol
  Validates MCP protocol implementation over HTTP transport

  Scenario: Initialize request returns protocol version
    Given the server is running
    When a POST request is made to /mcp with initialize method
    And the request contains valid JSON-RPC 2.0 structure
    And the request includes protocolVersion "2025-11-05"
    Then the response status should be 200 OK
    And the response should contain protocolVersion "2025-03-26"
    And the response should contain supported protocol versions

  Scenario: Initialized notification is accepted
    Given the server has been initialized
    When a POST request is made with notifications/initialized method
    Then the response status should be 202 Accepted
    And the response should have empty body

  Scenario: Invalid JSON-RPC version is rejected
    Given the server is running
    When a POST request is made with jsonrpc "1.0" instead of "2.0"
    Then the response status should indicate an error
    And the error should indicate version mismatch

  Scenario: Unknown method returns error
    Given the server is running
    When a POST request is made with unknown method "unknown/method"
    Then the response status should indicate method not found
    And the error response should contain correct JSON-RPC error format

  Scenario: GET request returns SSE stream
    Given the server is running
    When a GET request is made to /mcp
    Then the response status should be 200 OK
    And the Content-Type should start with "text/event-stream"

  Scenario: Missing protocol version header on non-initialize returns error
    Given the server is running
    When a POST request is made with invalid MCP-Protocol-Version header
    And the method is not initialize
    Then the response status should be 400 Bad Request
