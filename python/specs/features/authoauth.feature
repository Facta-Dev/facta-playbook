Feature: Social sign-in
  Google One Tap and the GitHub authorization-code flow log users in by
  verified external identity: link by google_id/github_id first, then by
  email, else provision an auto-verified user. Google answers the error
  envelope; GitHub never answers JSON — the callback always redirects to
  the frontend with a token fragment or an error query.

  Background:
    Given an ADMIN user "root@ex.com" with password "s3cure-pass-123"
    And a verified user "ada@ex.com" with password "s3cure-pass-123"
    And I log in as "root@ex.com" with password "s3cure-pass-123" and remember the pair as root

  Scenario: A valid Google credential returns the login shape
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 200
    And the response carries an access token and a refresh token

  Scenario: The Google identity links the existing email account, not a duplicate
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 200
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 200
    When I send "GET" "/api/v1/admin/users?page=1&page_size=20" as root
    Then the response field "total" is "2"

  Scenario: A never-seen Google identity provisions an auto-verified user
    When "fresh@ex.com" signs in with a Google credential
    Then the response status is 200
    And the response carries an access token and a refresh token
    And 0 "verify_email" emails are enqueued for "fresh@ex.com"
    When I send "GET" "/api/v1/admin/users?page=1&page_size=20" as root
    Then the response field "total" is "3"

  Scenario: An invalid Google credential is rejected
    When "ada@ex.com" signs in with an invalid Google credential
    Then the response status is 401
    And the response code is "AUTH_OAUTH_FAILED"

  Scenario: An unconfigured Google provider answers 503
    Given the google provider is not configured
    When "ada@ex.com" signs in with a Google credential
    Then the response status is 503
    And the response code is "AUTH_OAUTH_FAILED"

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
    When I send "GET" "/api/v1/admin/users?page=1&page_size=20" as root
    Then the response field "total" is "2"

  Scenario: Callback errors redirect with the error query, no tokens
    When I send "GET" "/api/v1/auth/github/callback?error=access_denied&error_description=denied"
    Then the response status is 302
    And the response redirects carrying error "access_denied"
    When I send "GET" "/api/v1/auth/github/callback"
    Then the response status is 302
    And the response redirects carrying error "auth.no_code"

  Scenario: A disabled GitHub provider answers 503 on the login route
    Given the github provider is disabled
    When I send "GET" "/api/v1/auth/github/login"
    Then the response status is 503
    And the response code is "AUTH_OAUTH_FAILED"
    When I send "GET" "/api/v1/auth/github/callback?code=good-code"
    Then the response status is 302
    And the response redirects carrying error "AUTH_OAUTH_FAILED"
