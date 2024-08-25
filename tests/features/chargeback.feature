Feature: Chargeback

  Scenario: Chargeback a deposit transaction
    Given A user has an empty account
    When the user deposits $100
    And the user disputes the last transaction
    And the the last disputed tx is charged back
    Then the user's balance should be $0

  Scenario: Fraudulent chargeback, negative balance
    Given A user has an empty account
    When the user deposits $100
    And the user withdraws $100
    And the user disputes the last deposit transaction
    And the the last disputed tx is charged back
    Then the last operation should succeed
    And the user's balance should be $-100
