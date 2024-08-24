Feature: Deposit

  Scenario: Single deposit
    Given A user has an account
    When the user deposits $100
    Then the user's available balance should be $100

  Scenario: Multiple deposits
    Given A user has an account
    When the user deposits $100
    And the user deposits $50
    And the user deposits $25
    Then the user's available balance should be $175