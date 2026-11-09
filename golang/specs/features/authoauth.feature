Feature: Social sign-in
  Google One Tap and the GitHub authorization-code flow log users in by
  verified external identity: link by google_id/github_id first, then by
  email, else provision an auto-verified user (default user_type COMPANY).
  Google answers the error envelope; GitHub never answers JSON — the
  callback always redirects to the frontend with a token fragment or an
  error query.

  Background:
    Given an ADMIN user "root@ex.com" with password "s3cure-pass-123"
    And a verified user "ada@ex.com" with password "s3cure-pass-123"
    And I log in as "root@ex.com" with password "s3cure-pass-123" and remember the pair as root

  Scenario: A valid Google ID token returns the login shape
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 200
    And the response has token_type "bearer" and expires_in 900

  Scenario: The Google identity links the existing email account, not a duplicate
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 200
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 200
    When I send "GET" "/api/v1/admin/users?limit=20&offset=0" as root
    Then the response field "total" is "2"

  Scenario: A never-seen Google identity provisions an auto-verified user
    When "fresh@ex.com" signs in with a Google credential
    Then the response status is 200
    And the response has token_type "bearer" and expires_in 900
    And 0 "verify_email" emails are enqueued for "fresh@ex.com"
    When I send "GET" "/api/v1/admin/users?limit=20&offset=0" as root
    Then the response field "total" is "3"

  Scenario: An invalid Google ID token is rejected
    When "ada@ex.com" signs in with an invalid Google credential
    Then the response status is 401
    And the response code is "AUTH_INVALID_TOKEN"

  Scenario: An unconfigured Google provider is a 401, not a crash
    Given the google provider is not configured
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 401

  Scenario: An inactive account cannot ride a social session
    Given an inactive user "gone@ex.com" with password "s3cure-pass-123"
    When "gone@ex.com" signs in with a Google credential
    Then the response status is 403
    And the response code is "AUTH_ACCOUNT_INACTIVE"

  Scenario: The GitHub login route redirects to the authorize URL
    When I send "GET" "/api/v1/auth/github/login"
    Then the response status is 302
    And the response redirects to a URL that starts with "https://github.com/login/oauth/authorize?"

  Scenario: The callback exchanges the code and redirects with the token pair
    When I send "GET" "/api/v1/auth/github/callback?code=good-code"
    Then the response status is 302
    And the response redirects carrying an access token

  Scenario: The GitHub identity links the existing email account, not a duplicate
    When I send "GET" "/api/v1/auth/github/callback?code=ada-code"
    Then the response status is 302
    And the response redirects carrying an access token
    When I send "GET" "/api/v1/admin/users?limit=20&offset=0" as root
    Then the response field "total" is "2"

  Scenario: Callback errors redirect with the error query, no tokens
    When I send "GET" "/api/v1/auth/github/callback?error=access_denied&error_description=denied"
    Then the response status is 302
    And the response redirects carrying error "access_denied"
    When I send "GET" "/api/v1/auth/github/callback"
    Then the response status is 302
    And the response redirects carrying error "no_code"

  Scenario: A disabled GitHub provider answers 403 on both routes
    Given the github provider is disabled
    When I send "GET" "/api/v1/auth/github/login"
    Then the response status is 403
    And the response code is "AUTH_FORBIDDEN"
    When I send "GET" "/api/v1/auth/github/callback?code=good-code"
    Then the response status is 403
    And the response code is "AUTH_FORBIDDEN"