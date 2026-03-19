Feature: MCP Resources
  Validates MCP resources implementation and resource reading

  Scenario: List resources returns available resource types
    Given the server is running
    When a resources/list request is made
    Then the response should contain a list of resource templates
    And the templates should include session, pane, and command resources

  Scenario: Read session resource returns session data
    Given the server is running
    And there are active tmux sessions
    When a resources/read request is made for tmux://sessions
    Then the response should contain session information
    And each session should have id, name and window count

  Scenario: Read pane resource returns pane content
    Given the server is running
    And there is an active pane with pane_id
    When a resources/read request is made for tmux://pane/{pane_id}
    Then the response should contain pane information
    And the content should include pane dimensions and process

  Scenario: Read command result resource returns command output
    Given a command has been executed with known command_id
    When a resources/read request is made for tmux://command/{command_id}/result
    Then the response should contain command output
    And the exit code should be captured if completed

  Scenario: Unknown resource URI returns error
    Given the server is running
    When a resources/read request is made for tmux://unknown/resource
    Then the response status should indicate not found
    And the error should indicate invalid URI

  Scenario: Pending command result shows pending status
    Given a command is currently executing
    When a resources/read request is made for its command result URI
    Then the response should indicate pending status
    And should not contain completed output

  Scenario: Completed command result shows exit code
    Given a command has completed with exit code 0
    When a resources/read request is made for its command result URI
    Then the response should contain the exit code 0
    And should contain command output if any
