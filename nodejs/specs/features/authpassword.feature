Feature: Password reset by email code, and password change
  A known email gets a 6-digit reset code (30 min, max 3 per 5 min);
  unknown emails answer success. Reset changes the password, auto-verifies
  the account and revokes all sessions. Change-password keeps the current
  session (by jti) and revokes the others.

  Background:
    Given a verified user "ada@ex.com" with password "old-pass-1234"

  Scenario: Forgot password for a known email queues the reset email
    When I ask to reset the password for "ada@ex.com"
    Then the response status is 200
    And 1 "reset_password" email was enqueued for "ada@ex.com"

  Scenario: Forgot password for an unknown email answers success without leak
    When I ask to reset the password for "ghost@ex.com"
    Then the response status is 200
    And 0 "reset_password" emails were enqueued for "ghost@ex.com"

  Scenario: Reset requests are rate limited after three in the window
    Given I asked to reset the password for "ada@ex.com" 3 times
    When I ask to reset the password for "ada@ex.com"
    Then the response status is 429
    And the response code is "AUTH_RATE_LIMITED"

  Scenario: A reset code can be validated without consuming it
    Given I requested a reset code for "ada@ex.com"
    When I validate "ada@ex.com" with the latest reset code
    Then the response status is 200
    When I validate "ada@ex.com" with the latest reset code
    Then the response status is 200
    When I validate "ada@ex.com" with a wrong reset code
    Then the response status is 400
    And the response code is "AUTH_CODE_INVALID"

  Scenario: An expired reset code is rejected
    Given I requested a reset code for "ada@ex.com"
    When 31 minutes pass
    When I reset the password for "ada@ex.com" using the latest reset code and password "new-pass-1234"
    Then the response status is 400
    And the response code is "AUTH_CODE_EXPIRED"

  Scenario: Reset sets the password and auto-verifies the account
    Given an unverified user "uno@ex.com" with password "old-pass-1234"
    And I log in as "uno@ex.com" with password "old-pass-1234"
    Then the response code is "AUTH_EMAIL_NOT_VERIFIED"
    Given I requested a reset code for "uno@ex.com"
    When I reset the password for "uno@ex.com" using the latest reset code and password "new-pass-1234"
    Then the response status is 200
    And 1 "password_changed" email was enqueued for "uno@ex.com"
    When I log in as "uno@ex.com" with password "new-pass-1234"
    Then the response status is 200

  Scenario: Reset revokes every active session
    When I log in as "ada@ex.com" with password "old-pass-1234" and remember the pair as first
    And I log in as "ada@ex.com" with password "old-pass-1234" and remember the pair as second
    And I requested a reset code for "ada@ex.com"
    When I reset the password for "ada@ex.com" using the latest reset code and password "new-pass-1234"
    Then the response status is 200
    When I refresh using the token "first"
    Then the response status is 401
    When I refresh using the token "second"
    Then the response status is 401

  Scenario: Change password refuses the wrong current password
    When I log in as "ada@ex.com" with password "old-pass-1234" and remember the pair as first
    And I change the password of first from "wrong-old-pass" to "new-pass-1234"
    Then the response status is 401
    And the response code is "AUTH_PASSWORD_WRONG"
    And 0 "password_changed" emails were enqueued for "ada@ex.com"

  Scenario: Change password keeps the current session and revokes the others
    When I log in as "ada@ex.com" with password "old-pass-1234" and remember the pair as first
    And I log in as "ada@ex.com" with password "old-pass-1234" and remember the pair as second
    And I change the password of second from "old-pass-1234" to "new-pass-1234"
    Then the response status is 200
    And 1 "password_changed" email was enqueued for "ada@ex.com"
    When I refresh using the token "second"
    Then the response status is 200
    When I refresh using the token "first"
    Then the response status is 401