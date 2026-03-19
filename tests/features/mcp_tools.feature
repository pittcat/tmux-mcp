Feature: MCP Tools
  Validates MCP tools implementation and tool calling

  Scenario: List tools returns tool manifest
    Given the server is running
    When a tools/list request is made
    Then the response should contain a list of available tools
    And each tool should have name, description and inputSchema

  Scenario: Execute list-sessions tool returns sessions
    Given the server is running
    When a tools/call request is made for list-sessions
    Then the response should contain session information
    And the sessions should have id, name and windows

  Scenario: Execute capture-pane tool returns pane content
    Given the server is running
    And there is an active pane with known pane_id
    When a tools/call request is made for capture-pane with pane_id
    Then the response should contain pane content
    And the content should include lines if available

  Scenario: Execute command tool starts command execution
    Given the server is running
    When a tools/call request is made for execute-command with a test command
    Then the response should contain a command_id
    And the command status should be "pending"

  Scenario: Get command result retrieves execution status
    Given a command has been executed with command_id
    When a resources/read request is made for tmux://command/{command_id}/result
    Then the response should contain command result or status
    And the status should indicate pending, completed, or error

  Scenario: Missing required parameter returns error
    Given the server is running
    When a tools/call request is made for capture-pane without pane_id
    Then the response should indicate missing required parameter
    And the error should indicate which parameter is missing

  Scenario: Split pane tool splits window
    Given the server is running
    And there is an active pane
    When a tools/call request is made for split-pane with direction "horizontal"
    Then the response should indicate success
    And a new pane should be created
