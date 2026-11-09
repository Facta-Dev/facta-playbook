Feature: Session management
  Verified and active users log in, refresh (rotation), log out and read
  their profile. Login gates and the constant invalid-credentials code
  never reveal whether an account exists.

  Background:
    Given a verified user "ada@ex.com" with password "s3cure-pass-123"

  Scenario: Login issues a bearer pair with a 15 minute access token
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    Then the response status is 200
    And the response has token_type "bearer" and expires_in 900

  Scenario: Wrong or unknown credentials answer one constant code
    When I log in as "ada@ex.com" with password "wrong-password"
    Then the response status is 401
    And the response code is "AUTH_INVALID_CREDENTIALS"
    When I log in as "ghost@ex.com" with password "wrong-password"
    Then the response status is 401
    And the response code is "AUTH_INVALID_CREDENTIALS"

  Scenario: The fifth failure locks the account
    When I log in as "ada@ex.com" with password "wrong-password" 5 times
    Then the response status is 403
    And the response code is "AUTH_ACCOUNT_BLOCKED"

  Scenario Outline: Sessions are gated by account state
    Given an <state> user "gated@ex.com" with password "s3cure-pass-123"
    When I log in as "gated@ex.com" with password "s3cure-pass-123"
    Then the response status is 403
    And the response code is "<code>"

    Examples:
      | state      | code                    |
      | unverified | AUTH_EMAIL_NOT_VERIFIED |
      | inactive   | AUTH_ACCOUNT_INACTIVE   |
      | blocked    | AUTH_ACCOUNT_BLOCKED    |

  Scenario: Refresh rotates the session and retires the old token
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I refresh using the token "ada"
    Then the response status is 200
    When I refresh using the token "ada"
    Then the response status is 401
    And the response code is "AUTH_INVALID_REFRESH"

  Scenario: Logout revokes the session
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I log out with the token "ada"
    Then the response status is 200
    When I refresh using the token "ada"
    Then the response status is 401

  Scenario: Me returns the profile of the caller, 401 without a bearer
    When I send "GET" "/api/v1/auth/me"
    Then the response status is 401
    And the response code is "AUTH_UNAUTHORIZED"
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I send "GET" "/api/v1/auth/me" as ada
    Then the response status is 200
    And the response field "email" is "ada@ex.com"
    And the response field "role" is "USER"

  Scenario: Update my profile keeps the fields I did not send
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I send "PUT" "/api/v1/auth/me" as ada with body:
      """
      {"first_name": "Ada", "phone_number": "+54 11 5555-5100"}
      """
    Then the response status is 200
    And the response field "first_name" is "Ada"
    And the response field "phone_number" is "+54 11 5555-5100"
    And the response field "language" is "en"
    When I send "GET" "/api/v1/auth/me" as ada
    Then the response field "first_name" is "Ada"
    And the response field "phone_number" is "+54 11 5555-5100"