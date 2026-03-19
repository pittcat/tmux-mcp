Feature: Runtime Resilience
  Validates timeout, concurrency, logging, and smoke scenarios

  Scenario: Command timeout handling
    Given the server is running
    When a command execution exceeds the timeout threshold
    Then the command should be terminated
    And an appropriate timeout error should be returned

  Scenario: Tmux not found error handling
    Given tmux is not installed or not in PATH
    When any tmux command is attempted
    Then an error should indicate tmux not found
    And the error type should be TmuxNotFound

  Scenario: Invalid session ID error
    Given the server is running
    When a command targets a non-existent session
    Then an error should indicate session not found
    And the session ID should be included in the error

  Scenario: Concurrent client connections
    Given the server is running
    When 10 clients connect simultaneously
    And each client makes requests
    Then all requests should be handled correctly
    And no data corruption should occur

  Scenario: Command registry capacity limit
    Given the command registry is at capacity
    When a new command is inserted
    Then the oldest expired command should be evicted
    And the new command should be stored

  Scenario: Command registry TTL expiration
    Given a command older than TTL exists
    When cleanup runs
    Then expired commands should be removed
    And active commands should be preserved

  Scenario: Log rotation with hourly intervals
    Given the server has been running for more than an hour
    When log rotation occurs
    Then new log file should be created
    And old log lines should be pruned after 4 hours

  Scenario: Log cleanup only affects expired files
    Given multiple log files with different ages
    When cleanup runs
    Then only files older than retention period should be deleted
    And recent log files should be preserved

  Scenario: Real tmux smoke test
    Given a running tmux server
    When I create a test session with test prefix
    And I create a window in that session
    And I split a pane
    Then all operations should succeed
    And cleanup should remove only test-prefixed resources
