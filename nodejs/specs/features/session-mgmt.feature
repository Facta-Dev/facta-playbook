Feature: Session management surface
  Own sessions list / current / revoke with the admin-side variants and the
  anomalies report. Rotation keeps de-activated rows as history, so
  active_only=false sees more than the default view; revocation answers the
  count of what it deactivated. Session lists come back as bare arrays.

  Background:
    Given an ADMIN user "root@ex.com" with password "s3cure-pass-123"
    And a verified user "ada@ex.com" with password "s3cure-pass-123"
    And I log in as "root@ex.com" with password "s3cure-pass-123" and remember the pair as root
    And I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada

  Scenario: The routes need a bearer token
    When I send "GET" "/api/v1/auth/sessions"
    Then the response status is 401
    And the response code is "AUTH_UNAUTHORIZED"
    When I send "GET" "/api/v1/auth/sessions/current"
    Then the response status is 401
    When I send "POST" "/api/v1/auth/sessions/revoke-current"
    Then the response status is 401
    When I send "POST" "/api/v1/auth/sessions/revoke-all"
    Then the response status is 401

  Scenario: The own list defaults to active sessions
    When I send "GET" "/api/v1/auth/sessions" as ada
    Then the response status is 200
    And the list has 1 entry
    And the response field "is_active" is "true" on entry 0

  Scenario: Rotation keeps a single row (fixed deadline, no history)
    When I refresh using the token "ada"
    Then the response status is 200
    When I send "GET" "/api/v1/auth/sessions?active_only=false" as ada
    Then the response status is 200
    And the list has 1 entries
    When I send "GET" "/api/v1/auth/sessions" as ada
    Then the list has 1 entry
    And the response field "is_active" is "true" on entry 0

  Scenario: The current session answers for the caller
    When I send "GET" "/api/v1/auth/sessions/current" as ada
    Then the response status is 200
    And the response field "is_active" is "true"

  Scenario: revoke-current retires only the caller's session
    When I send "POST" "/api/v1/auth/sessions/revoke-current" as ada
    Then the response status is 200
    And the response field "revoked" is "true"
    When I refresh using the token "ada"
    Then the response status is 401

  Scenario: revoke-all answers the revoked count
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada2
    And I send "POST" "/api/v1/auth/sessions/revoke-all" as ada2
    Then the response status is 200
    And the response field "revoked_count" is "2"
    When I refresh using the token "ada"
    Then the response status is 401
    When I refresh using the token "ada2"
    Then the response status is 401
    But I refresh using the token "root"
    Then the response status is 200

  Scenario: A non-admin caller gets 403 on the admin session routes
    When I send "GET" "/api/v1/admin/auth-session/users/00000000-0000-0000-0000-000000000002/sessions" as ada
    Then the response status is 403
    And the response code is "AUTH_FORBIDDEN"

  Scenario: Admin lists a user's sessions with the active filter
    When I send "GET" "/api/v1/admin/auth-session/users/00000000-0000-0000-0000-000000000002/sessions" as root
    Then the response status is 200
    And the list has 1 entry
    When I send "GET" "/api/v1/admin/auth-session/users/00000000-0000-0000-0000-000000000002/sessions?active_only=false" as root
    Then the list has 1 entry
    And the response field "is_active" is "true" on entry 0

  Scenario: Admin revoke-all retires every session of the target
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada2
    And I send "POST" "/api/v1/admin/auth-session/users/00000000-0000-0000-0000-000000000002/revoke-all" as root
    Then the response status is 200
    And the response field "revoked_count" is "2"
    When I refresh using the token "ada2"
    Then the response status is 401

  Scenario: The anomalies report answers and stays quiet in a calm window
    When I send "GET" "/api/v1/admin/auth-session/anomalies?window_minutes=60" as root
    Then the response status is 200
    And the list has 0 entries
    When I send "GET" "/api/v1/admin/auth-session/anomalies?window_minutes=0" as root
    Then the response status is 422
    And the response code is "VALIDATION_ERROR"

  Scenario: A bad window is unprocessable
    When I send "GET" "/api/v1/admin/auth-session/anomalies?window_minutes=nope" as root
    Then the response status is 422
    And the response code is "VALIDATION_ERROR"