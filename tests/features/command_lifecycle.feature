Feature: Command Lifecycle
  Validates command execution state machine and modes

  Scenario: Execute command creates pending command
    Given the server is running
    When I execute "echo test" via execute-command
    Then a command_id should be returned
    And the command status should be "pending"

  Scenario: Check pending command status
    Given a pending command with known command_id
    When I check the command status
    Then the status should be "pending"
    And there should be no exit code yet

  Scenario: Command completion with exit code 0
    Given a command that completes with exit code 0
    When I check the command status
    Then the exit_code should be 0
    And the output should contain command result

  Scenario: Command completion with non-zero exit code
    Given a command that fails with exit code 1
    When I check the command status
    Then the exit_code should be 1
    And the output should contain error information

  Scenario: Raw mode sends keys without processing
    Given the server is running
    When I execute a command in raw mode
    Then the command should be sent without markers
    And status tracking should be disabled

  Scenario: No-enter mode sends keystrokes without Enter
    Given the server is running
    When I send keys without pressing Enter via noEnter mode
    Then the keys should be sent character by character
    And no command execution should occur

  Scenario: Missing marker in command output
    Given a command that completes but output lacks markers
    When I check the command status
    Then appropriate error should indicate missing marker
    And the command should be marked as error state

  Scenario: Marker order error detection
    Given a command with malformed output (markers in wrong order)
    When I check the command status
    Then error should indicate marker sequence issue
