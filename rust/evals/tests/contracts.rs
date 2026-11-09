//! Contract tests for the template's .claude-layer evals.
//!
//! Ports the gtm valid+invalid fixture pair + regex-match-on-failure pattern
//! into the native `cargo test` runner. Each invalid fixture exercises a
//! distinct failure mode; the test asserts the validator rejects it and that
//! the joined error message contains a known substring (unanchored, mirroring
//! node's `assert.match`).

use claude_layer_evals::{Kind, ValidationResult, read_fixture, validate};

/// Helper: assert a valid fixture passes (ok == true, no errors).
fn assert_pass(result: &ValidationResult, ctx: &str) {
    assert!(result.ok, "{ctx}: expected ok, got errors: {:?}", result.errors);
    assert!(result.errors.is_empty(), "{ctx}: expected no errors, got: {:?}", result.errors);
}

/// Helper: assert an invalid fixture fails and the joined errors contain `pat`.
fn assert_fail_matching(result: &ValidationResult, pat: &str, ctx: &str) {
    assert!(!result.ok, "{ctx}: expected failure, but validator passed");
    let joined = result.errors.join("\n");
    assert!(
        joined.contains(pat),
        "{ctx}: expected errors to contain '{pat}', got:\n{joined}"
    );
}

#[test]
fn valid_skill_fixture_passes() {
    let text = read_fixture("fixtures/valid/skill.md");
    let result = validate(&text, Kind::Skill);
    assert_pass(&result, "valid skill");
}

#[test]
fn valid_agent_fixture_passes() {
    let text = read_fixture("fixtures/valid/agent.md");
    let result = validate(&text, Kind::Agent);
    assert_pass(&result, "valid agent");
}

#[test]
fn valid_command_fixture_passes() {
    let text = read_fixture("fixtures/valid/command.md");
    let result = validate(&text, Kind::Command);
    assert_pass(&result, "valid command");
}

#[test]
fn invalid_missing_frontmatter_key_fails() {
    let text = read_fixture("fixtures/invalid/missing-frontmatter-key.md");
    let result = validate(&text, Kind::Skill);
    assert_fail_matching(&result, "missing frontmatter key 'division'", "missing-frontmatter-key");
}

#[test]
fn invalid_bad_division_fails() {
    let text = read_fixture("fixtures/invalid/bad-division.md");
    let result = validate(&text, Kind::Skill);
    assert_fail_matching(&result, "bad division 'frontend'", "bad-division");
}

#[test]
fn invalid_missing_body_section_fails() {
    let text = read_fixture("fixtures/invalid/missing-body-section.md");
    let result = validate(&text, Kind::Skill);
    assert_fail_matching(&result, "missing body section 'Output Format'", "missing-body-section");
}

#[test]
fn invalid_literal_placeholder_leak_fails() {
    let text = read_fixture("fixtures/invalid/literal-placeholder-leak.md");
    let result = validate(&text, Kind::Skill);
    assert_fail_matching(&result, "literal placeholder leak", "literal-placeholder-leak");
}

#[test]
fn empty_frontmatter_fails() {
    let text = "---\n---\n\n# body\n";
    let result = validate(text, Kind::Skill);
    assert_fail_matching(&result, "empty frontmatter", "empty-frontmatter");
}

#[test]
fn no_frontmatter_fails() {
    let text = "# No frontmatter here\n\n## When to Activate\n";
    let result = validate(text, Kind::Skill);
    assert_fail_matching(&result, "missing or malformed frontmatter", "no-frontmatter");
}
