Feature: Withdrawal

  Scenario: Single withdrawal, has enough funds
    Given A user has an account with $100
    When the user withdraws $100
    Then the user's balance should be $0

  Scenario: Single withdrawal, empty account, insufficient funds
    Given A user has an empty account
    When the user withdraws $200
    Then the last operation should fail
    And the user's balance should be unchanged

  Scenario: Single withdrawal, has some funds, insufficient funds
    Given A user has an account with $100
    When the user withdraws $200
    Then the last operation should fail
    And the user's balance should be unchanged

  Scenario: Multiple withdrawals
    Given A user has an account with $100
    When the user withdraws $50
    And the user withdraws $25
    And the user withdraws $25
    Then the user's balance should be $0

  Scenario: Multiple withdrawals, insufficient funds
    Given A user has an account with $100
    When the user withdraws $50
    And the user withdraws $30
    And the user withdraws $75
    Then the last operation should fail
    And the user's balance should be $20

  Scenario: Withdrawal zero should fail
    Given A user has an account with $100
    When the user withdraws $0
    Then the last operation should fail
    And the user's balance should be $100
