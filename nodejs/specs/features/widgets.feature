Feature: Widgets CRUD and events
  Every widget route needs a bearer and is owner-scoped: one user never
  sees another user's widgets, including through the event stream.

  Background:
    Given a verified user "ada@ex.com" with password "s3cure-pass-123"
    And a verified user "ben@ex.com" with password "s3cure-pass-123"
    And I log in as "ada@ex.com" with password "s3cure-pass-123" and remember the pair as ada
    And I log in as "ben@ex.com" with password "s3cure-pass-123" and remember the pair as ben

  Scenario: Widget routes require a bearer token
    When I send "GET" "/api/v1/widgets"
    Then the response status is 401
    And the response code is "AUTH_UNAUTHORIZED"

  Scenario: Create, read, update, then delete a widget
    When I send "POST" "/api/v1/widgets" as ada with body:
      """
      {"name": "gizmo", "kind": "sprocket"}
      """
    Then the response status is 201
    And the response field "name" is "gizmo"
    And I store the response field "id" as widget_id
    When I send "GET" "/api/v1/widgets" as ada
    Then the response lists 1 item
    When I send "GET" "/api/v1/widgets/{{widget_id}}" as ada
    Then the response field "name" is "gizmo"
    When I send "PUT" "/api/v1/widgets/{{widget_id}}" as ada with body:
      """
      {"name": "mega-gizmo", "kind": "cog"}
      """
    Then the response status is 200
    And the response field "name" is "mega-gizmo"
    When I send "DELETE" "/api/v1/widgets/{{widget_id}}" as ada
    Then the response status is 204
    When I send "GET" "/api/v1/widgets/{{widget_id}}" as ada
    Then the response status is 404
    And the response code is "NOT_FOUND"

  Scenario: Widgets are scoped to their owner
    When I send "POST" "/api/v1/widgets" as ada with body:
      """
      {"name": "ada-only", "kind": "sprocket"}
      """
    Then the response status is 201
    And I store the response field "id" as widget_id
    When I send "GET" "/api/v1/widgets" as ben
    Then the response lists 0 items
    When I send "GET" "/api/v1/widgets/{{widget_id}}" as ben
    Then the response status is 404

  Scenario: The owner's event stream sees the created widget
    When I open the event stream for "ada"
    And I send "POST" "/api/v1/widgets" as ada with body:
      """
      {"name": "streamed", "kind": "sprocket"}
      """
    Then the response status is 201
    And the event stream of "ada" shows a "widget.created" event