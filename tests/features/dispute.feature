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
    Then the user's balance should be $0