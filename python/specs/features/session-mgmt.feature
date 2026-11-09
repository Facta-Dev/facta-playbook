Feature: Session management surface
  Own sessions list / current / revoke with the admin-side variants and the
  anomalies report. The own list is a bare array of the caller's active
  sessions, and the current one is flagged only when its refresh token is
  presented. Revocation answers the count of what it deactivated.

  Background:
    Given an ADMIN user "root@ex.com" with password "s3cure-pass-123"
    And a verified user "ada@ex.com" with password "s3cure-pass-123"
    And I log in as "root@ex.com" with password "s3cure-pass-123" and remember the pair as root
    And I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada

  Scenario: The own routes need a bearer token
    When I send "GET" "/api/v1/auth/sessions"
    Then the response status is 401
    And the response code is "UNAUTHENTICATED"
    When I send "GET" "/api/v1/auth/sessions/current"
    Then the response status is 401
    When I send "POST" "/api/v1/auth/sessions/revoke-all"
    Then the response status is 401

  Scenario: The own list shows the caller's active sessions
    When I send "GET" "/api/v1/auth/sessions" as ada
    Then the response status is 200
    And the list has 1 entry
    And the response field "current" is "false" on entry 0

  Scenario: The current session is flagged when its refresh token is presented
    When I remember the refresh token of "ada" as ada_refresh
    And I send "GET" "/api/v1/auth/sessions?refresh_token={{ada_refresh}}" as ada
    Then the response status is 200
    And the response field "current" is "true" on entry 0

  Scenario: The current session answers for the caller
    When I send "GET" "/api/v1/auth/sessions/current" as ada
    Then the response status is 200
    And the response field "ip" is "127.0.0.1"

  Scenario: revoke-current retires only the presented session
    When I remember the refresh token of "ada" as ada_refresh
    And I send "POST" "/api/v1/auth/sessions/revoke-current" as ada with body:
      """
      {"refresh_token": "{{ada_refresh}}"}
      """
    Then the response status is 200
    And the response field "revoked" is "1"
    When I refresh using the token "ada"
    Then the response status is 401

  Scenario: revoke-all answers the revoked count
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada2
    And I send "POST" "/api/v1/auth/sessions/revoke-all" as ada2
    Then the response status is 200
    And the response field "revoked" is "2"
    When I refresh using the token "ada"
    Then the response status is 401
    When I refresh using the token "ada2"
    Then the response status is 401
    But I refresh using the token "root"
    Then the response status is 200

  Scenario: A non-admin caller gets 403 on the admin session routes
    When I send "GET" "/api/v1/admin/auth-session/users/00000000-0000-0000-0000-000000000002/sessions" as ada
    Then the response status is 403
    And the response code is "FORBIDDEN"

  Scenario: Admin lists a user's sessions
    When I send "GET" "/api/v1/admin/auth-session/users/00000000-0000-0000-0000-000000000002/sessions" as root
    Then the response status is 200
    And the list has 1 entry
    And the response field "current" is "false" on entry 0

  Scenario: Admin revoke-all retires every session of the target
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada2
    And I send "POST" "/api/v1/admin/auth-session/users/00000000-0000-0000-0000-000000000002/revoke-all" as root
    Then the response status is 200
    And the response field "revoked" is "2"
    When I refresh using the token "ada2"
    Then the response status is 401

  Scenario: The anomalies report stays quiet in a calm window
    When I send "GET" "/api/v1/admin/auth-session/anomalies" as root
    Then the response status is 200
    And the list has 0 entries

  Scenario: The anomalies report flags a user seen from many addresses
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada3 from the address "10.0.0.9"
    And I send "GET" "/api/v1/admin/auth-session/anomalies" as root
    Then the response status is 200
    And the list has 1 entry
    And the response field "distinct_ips" is "2" on entry 0
