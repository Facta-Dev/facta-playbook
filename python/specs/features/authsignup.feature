Feature: Signup and email verification
  Unverified users sign up and receive a 6-digit verification code by email
  (24h). Re-signup for an unverified email re-issues the code; a verified
  email is a 409 conflict. Resend never leaks whether an email exists.

  Scenario: Signup creates an unverified user and queues the verification email
    When I sign up "ada@ex.com" with password "s3cure-pass-123"
    Then the response status is 201
    And 1 "verify_email" email was enqueued for "ada@ex.com"

  Scenario: Re-signup for an unverified email re-issues the code
    Given "ada@ex.com" has signed up but not verified
    When I sign up "ada@ex.com" with password "s3cure-pass-123"
    Then the response status is 201
    And 2 "verify_email" emails were enqueued for "ada@ex.com"

  Scenario: Signup for a verified email answers 409
    Given a verified user "ada@ex.com" with password "s3cure-pass-123"
    When I sign up "ada@ex.com" with password "s3cure-pass-123"
    Then the response status is 409
    And the response code is "AUTH_EMAIL_TAKEN"

  Scenario: A valid code verifies the email and the replay is rejected
    Given "ada@ex.com" has signed up but not verified
    When I verify "ada@ex.com" with the latest emailed code
    Then the response status is 200
    When I verify "ada@ex.com" with the latest emailed code
    Then the response status is 400
    And the response code is "AUTH_CODE_INVALID"

  Scenario: A wrong code is rejected
    Given "ada@ex.com" has signed up but not verified
    When I verify "ada@ex.com" with a wrong code
    Then the response status is 400
    And the response code is "AUTH_CODE_INVALID"

  Scenario: An expired code is rejected
    Given "ada@ex.com" has signed up but not verified
    When 25 hours pass
    And I verify "ada@ex.com" with the latest emailed code
    Then the response status is 400
    And the response code is "AUTH_CODE_EXPIRED"

  Scenario: Resend answers success without leaking account existence
    When I resend verification for "ghost@ex.com"
    Then the response status is 202
    And 0 "verify_email" emails were enqueued for "ghost@ex.com"
    Given a verified user "sam@ex.com" with password "s3cure-pass-123"
    When I resend verification for "sam@ex.com"
    Then the response status is 409
    And the response code is "AUTH_EMAIL_TAKEN"
