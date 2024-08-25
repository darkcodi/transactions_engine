Feature: Resolve

  Scenario: Resolve a dispute
    Given A user has an empty account
    When the user deposits $100
    And the user disputes the last transaction
    And the the last disputed tx is resolved
    Then the user's balance should be $100
