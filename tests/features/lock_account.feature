Feature: Lock account after chargeback

  Scenario: No lock after resolving a dispute
    Given A user has an empty account
    When the user deposits $100
    And the user disputes the last transaction
    And the the last disputed tx is resolved
    Then the user's account should not be locked

  Scenario: Lock after chargeback
    Given A user has an empty account
    When the user deposits $100
    And the user disputes the last transaction
    And the the last disputed tx is charged back
    Then the user's account should be locked

  Scenario: No withdrawal after lock
    Given A user has an locked account with $100
    When the user withdraws $50
    Then the last operation should fail
    And the user's balance should be $100

  Scenario: No deposit after lock
    Given A user has an locked account with $100
    When the user deposits $50
    Then the last operation should fail
    And the user's balance should be $100
