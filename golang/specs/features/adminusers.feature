Feature: Admin user management
  ADMIN callers list, inspect and update users; every mutation lands in
  audit_logs. Non-admins are forbidden. Seeded ids are deterministic: the
  Background seeds root first (…001) and ada second (…002).

  Background:
    Given an ADMIN user "root@ex.com" with password "s3cure-pass-123"
    And a verified user "ada@ex.com" with password "s3cure-pass-123"
    And I log in as "root@ex.com" with password "s3cure-pass-123" and remember the pair as root

  Scenario: A non-admin caller gets 403 on admin routes
    When I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I send "GET" "/api/v1/admin/users" as ada
    Then the response status is 403
    And the response code is "AUTH_FORBIDDEN"

  Scenario: The list is paginated with a total
    When I send "GET" "/api/v1/admin/users?limit=1&offset=0" as root
    Then the response status is 200
    And the response lists 1 item
    And the response field "total" is "2"
    When I send "GET" "/api/v1/admin/users?limit=20&offset=0" as root
    Then the response status is 200
    And the response lists 2 items

  Scenario: Fetching a user, known and unknown
    When I send "GET" "/api/v1/admin/users/00000000-0000-0000-0000-000000000001" as root
    Then the response status is 200
    And the response field "email" is "root@ex.com"
    When I send "GET" "/api/v1/admin/users/00000000-0000-0000-0000-000000000999" as root
    Then the response status is 404
    And the response code is "NOT_FOUND"

  Scenario: Blocking a user takes effect and is audited
    When I send "PUT" "/api/v1/admin/users/00000000-0000-0000-0000-000000000002" as root with body:
      """
      {"is_blocked": true}
      """
    Then the response status is 200
    And the response field "is_blocked" is "true"
    And 1 audit entry was recorded
    When I log in as "ada@ex.com" with password "s3cure-pass-123"
    Then the response status is 403
    And the response code is "AUTH_ACCOUNT_BLOCKED"

  Scenario Outline: Updating a role accepts the shared vocabulary
    When I send "PUT" "/api/v1/admin/users/00000000-0000-0000-0000-000000000002" as root with body:
      """
      {"role": "<role>"}
      """
    Then the response status is 200
    And the response field "role" is "<role>"

    Examples:
      | role      |
      | MODERATOR |
      | ADMIN     |

  Scenario: A bad role is unprocessable and an unknown id stays unaudited
    When I send "PUT" "/api/v1/admin/users/00000000-0000-0000-0000-000000000002" as root with body:
      """
      {"role": "WIZARD"}
      """
    Then the response status is 422
    And the response code is "VALIDATION_ERROR"
    And 0 audit entries were recorded
    When I send "PUT" "/api/v1/admin/users/00000000-0000-0000-0000-000000000999" as root with body:
      """
      {"is_active": false}
      """
    Then the response status is 404
    And 0 audit entries were recorded

  Scenario: Upgrade-to-admin bootstrap follows the whitelist
    When I send "POST" "/api/v1/auth/upgrade-to-admin" with body:
      """
      {"email": "ada@ex.com"}
      """
    Then the response status is 403
    And the response code is "AUTH_FORBIDDEN"
    # On the whitelist but no account row yet: still 404, and the whitelist
    # check precedes lookup so unknown emails leak nothing.
    When I send "POST" "/api/v1/auth/upgrade-to-admin" with body:
      """
      {"email": "admin-boost@ex.com"}
      """
    Then the response status is 404
    And the response code is "NOT_FOUND"
    Given a verified user "admin-boost@ex.com" with password "s3cure-pass-123"
    # The whitelist matches case-insensitively; the response lowercases.
    When I send "POST" "/api/v1/auth/upgrade-to-admin" with body:
      """
      {"email": "Admin-Boost@ex.com"}
      """
    Then the response status is 200
    And the response field "upgraded" is "true"
    And the response field "email" is "admin-boost@ex.com"
    When I send "POST" "/api/v1/auth/upgrade-to-admin" with body:
      """
      {"email": "admin-boost@ex.com"}
      """
    Then the response status is 200
    And the response field "upgraded" is "false"
    And the response field "message" is "User is already an admin"
    When I log in as "admin-boost@ex.com" with password "s3cure-pass-123" and remember the pair as boost
    When I send "GET" "/api/v1/admin/users" as boost
    Then the response status is 200