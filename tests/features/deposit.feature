Feature: Deposit

  Scenario: Single deposit
    Given A user has an empty account
    When the user deposits $100
    Then the user's balance should be $100

  Scenario: Multiple deposits
    Given A user has an empty account
    When the user deposits $100
    And the user deposits $50
    And the user deposits $25
    Then the user's balance should be $175

  Scenario: Deposit zero should fail
    Given A user has an empty account
    When the user deposits $0
    Then the last operation should fail
    And the user's balance should be $0
