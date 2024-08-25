Feature: Dispute

  Scenario: Dispute a deposit transaction
    Given A user has an empty account
    When the user deposits $100
    And the user disputes the last transaction
    Then the user's available balance should be $0
    And the user's held balance should be $100

  Scenario: Can not dispute a withdrawal transaction
    Given A user has an account with $100
    When the user withdraws $100
    And the user disputes the last transaction
    Then the last operation should fail
    And the user's balance should be $0

  Scenario: Dispute deposit after withdrawal transaction
    Given A user has an empty account
    When the user deposits $100
    And the user withdraws $100
    And the user disputes the last deposit transaction
    Then the last operation should succeed
    And the user's available balance should be $-100
    And the user's held balance should be $100
