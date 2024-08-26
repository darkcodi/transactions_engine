Feature: CSV input

  Scenario: Read CSV
      Given the CSV file with the following content:
      """
        type, client, tx, amount
        deposit, 1, 1, 1.0
        deposit, 2, 2, 2.0
        deposit, 1, 3, 2.0
        withdrawal, 1, 4, 1.5
        withdrawal, 2, 5, 3.0
      """
      When the CSV operations are performed
      Then the accounts should be as follows:
      | client | available | held | total | locked |
      | 1      | 1.5       | 0.0  | 1.5   | false  |
      | 2      | 2.0       | 0.0  | 2.0   | false  |
