Feature: Session management
  Verified and active users log in, refresh (rotation), log out and read
  their profile. Login gates and the constant invalid-credentials code
  never reveal whether an account exists.

  Background:
    Given a verified user "ada@ex.com" with password "s3cure-pass-123"

  Scenario: Login issues a bearer pair
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    Then the response status is 200
    And the response carries an access token and a refresh token

  Scenario: Wrong or unknown credentials answer one constant code
    When I log in as "ada@ex.com" with password "wrong-password"
    Then the response status is 401
    And the response code is "AUTH_INVALID_CREDENTIALS"
    When I log in as "ghost@ex.com" with password "wrong-password"
    Then the response status is 401
    And the response code is "AUTH_INVALID_CREDENTIALS"

  Scenario Outline: Sessions are gated by account state
    Given <article> <state> user "gated@ex.com" with password "s3cure-pass-123"
    When I log in as "gated@ex.com" with password "s3cure-pass-123"
    Then the response status is 403
    And the response code is "<code>"

    Examples:
      | article | state      | code                    |
      | an      | unverified | AUTH_EMAIL_UNVERIFIED   |
      | an      | inactive   | AUTH_ACCOUNT_INACTIVE   |
      | a       | blocked    | AUTH_ACCOUNT_BLOCKED    |

  Scenario: Refresh rotates the session and retires the old token
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I refresh using the token "ada"
    Then the response status is 200
    When I refresh using the token "ada"
    Then the response status is 401
    And the response code is "AUTH_INVALID_TOKEN"

  Scenario: Logout revokes the session
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I log out with the token "ada"
    Then the response status is 204
    When I refresh using the token "ada"
    Then the response status is 401

  Scenario: Me returns the profile of the caller, 401 without a bearer
    When I send "GET" "/api/v1/auth/me"
    Then the response status is 401
    And the response code is "UNAUTHENTICATED"
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I send "GET" "/api/v1/auth/me" as ada
    Then the response status is 200
    And the response field "email" is "ada@ex.com"
    And the response field "role" is "user"

  Scenario: Update my profile keeps the fields I did not send
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I send "PATCH" "/api/v1/auth/me" as ada with body:
      """
      {"first_name": "Ada", "language": "es"}
      """
    Then the response status is 200
    And the response field "first_name" is "Ada"
    And the response field "language" is "es"
    And the response field "last_name" is ""
    When I send "GET" "/api/v1/auth/me" as ada
    Then the response field "first_name" is "Ada"
    And the response field "language" is "es"
