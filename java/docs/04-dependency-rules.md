# Dependency rules

- Maven modules + Enforcer ban a forbidden artifact; the reactor fails an undeclared module dependency.
- ArchUnit asserts domain has no `jakarta.*`/Hibernate import, the resource has no repository, and slices do not import each other.
- `mvn verify` is the gate; a violation is not a review comment.
