//! claude-layer-evals — pure-std contract validator for the `.claude` layer.
//!
//! Hand-rolled frontmatter parser + body-section checker for agents, skills,
//! and commands. Zero external dependencies: no serde, no serde_yaml, no
//! markdown crate. The required fields, required sections, allowed divisions,
//! and failure modes are HARDCODED as static arrays in
//! `required_frontmatter()` / `required_sections()` / `allowed_divisions()`
//! below — this validator does NOT read `.claude/schemas/*.schema.md` at
//! runtime. The schema docs are the source of truth and the bash linters
//! (`scripts/lint-skills.sh`, `scripts/lint-commands.sh`) parse them at runtime,
//! so the linter<->schema drift guarantee holds via the linters. This
//! validator is a hand-maintained mirror of the schema docs for the native
//! `cargo test` runner: when a list changes in a schema doc you MUST update
//! the matching static array here too.
//!
//! The validator is the contract surface exercised by `tests/contracts.rs`
//! over `fixtures/valid/*.md` and `fixtures/invalid/*.md`. Each invalid
//! fixture exercises a distinct failure mode; the test asserts the validator
//! rejects it and that the error message matches a known regex (porting the
//! gtm valid+invalid fixture pair + regex-match-on-failure pattern).

#![deny(unsafe_code)]
#![forbid(unsafe_code)]

/// Which `.claude` module kind a document belongs to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    Agent,
    Skill,
    Command,
}

impl Kind {
    fn required_frontmatter(&self) -> &'static [&'static str] {
        match self {
            Kind::Agent => &["name", "description", "color", "division"],
            Kind::Skill | Kind::Command => &["name", "description", "division"],
        }
    }

    fn required_sections(&self) -> &'static [&'static str] {
        match self {
            Kind::Agent => &[
                "Core Mission",
                "Expertise Areas",
                "Output Format",
                "Quality Standards",
                "Guiding Principles",
            ],
            Kind::Skill => &[
                "When to Activate",
                "Core Principles",
                "Rules",
                "Research References",
                "Output Format",
            ],
            Kind::Command => &["Delegate"],
        }
    }

    fn allowed_divisions(&self) -> &'static [&'static str] {
        // Shared across agents/skills/commands in this template.
        &["backend", "security", "testing", "core"]
    }
}

/// Result of validating one document.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ValidationResult {
    pub ok: bool,
    pub errors: Vec<String>,
}

impl ValidationResult {
    fn pass() -> Self {
        Self { ok: true, errors: Vec::new() }
    }
    fn push(&mut self, err: impl Into<String>) {
        self.ok = false;
        self.errors.push(err.into());
    }
}

/// Parse a YAML frontmatter block into an ordered map of key -> value.
///
/// Accepts the leading `---\n` fence, reads until the closing `---`, and
/// parses `key: value` lines (stripping surrounding single/double quotes from
/// the value). Returns `(frontmatter, body, rest_ok)` where `rest_ok` is false
/// if the frontmatter is missing or malformed.
pub fn parse_frontmatter(text: &str) -> (Vec<(String, String)>, String, bool) {
    let mut lines = text.lines();
    let first = lines.next();
    if first.map(str::trim) != Some("---") {
        return (Vec::new(), text.to_owned(), false);
    }
    let mut fm: Vec<(String, String)> = Vec::new();
    let mut body_lines: Vec<String> = Vec::new();
    let mut closed = false;
    for line in lines {
        if !closed {
            if line.trim() == "---" {
                closed = true;
                continue;
            }
            // Skip blank lines and comments inside frontmatter.
            let t = line.trim();
            if t.is_empty() || t.starts_with('#') {
                continue;
            }
            if let Some((k, v)) = line.split_once(':') {
                let key = k.trim().to_owned();
                let mut val = v.trim().to_owned();
                // Strip one layer of matching surrounding quotes.
                if val.len() >= 2 {
                    let b = val.as_bytes();
                    let q = b[0];
                    if (q == b'"' || q == b'\'') && b[b.len() - 1] == q {
                        val = val[1..val.len() - 1].to_owned();
                    }
                }
                fm.push((key, val));
            }
        } else {
            body_lines.push(line.to_owned());
        }
    }
    let body = body_lines.join("\n");
    (fm, body, closed)
}

/// Look up a frontmatter value by key (first match).
pub fn fm_get<'a>(fm: &'a [(String, String)], key: &str) -> Option<&'a str> {
    fm.iter().find(|(k, _)| k == key).map(|(_, v)| v.as_str())
}

/// Check whether the body contains a heading matching `section`
/// (case-insensitive), accepting `#`, `##`, or `###` prefixes.
pub fn has_section(body: &str, section: &str) -> bool {
    let needle = section.to_ascii_lowercase();
    for line in body.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix('#') {
            let heading = rest.trim_start_matches('#').trim();
            if heading.to_ascii_lowercase() == needle {
                return true;
            }
        }
    }
    false
}

/// Check for residual literal placeholders (`<app>`, `<APP>`, `<scope>`).
pub fn has_placeholder_leak(text: &str) -> bool {
    text.contains("<app>") || text.contains("<APP>") || text.contains("<scope>")
}

/// Validate a single document against the rules for `kind`.
pub fn validate(text: &str, kind: Kind) -> ValidationResult {
    let mut result = ValidationResult::pass();

    // CRLF check.
    if text.contains('\r') {
        result.push("CRLF line endings — convert to LF");
    }

    let (fm, body, closed) = parse_frontmatter(text);
    if !closed {
        result.push("missing or malformed frontmatter (no closing ---)");
        return result;
    }
    if fm.is_empty() {
        result.push("empty frontmatter");
    }

    // Required frontmatter fields.
    for field in kind.required_frontmatter() {
        if fm_get(&fm, field).is_none() {
            result.push(format!("missing frontmatter key '{field}'"));
        }
    }

    // Division membership in the allowed enum.
    if let Some(div) = fm_get(&fm, "division") {
        if !kind.allowed_divisions().contains(&div) {
            result.push(format!("bad division '{div}'"));
        }
    }

    // Required body sections.
    for section in kind.required_sections() {
        if !has_section(&body, section) {
            result.push(format!("missing body section '{section}'"));
        }
    }

    // Literal placeholder leak (post-rename contract: 0 residual).
    if has_placeholder_leak(text) {
        result.push("literal placeholder leak (<app>/<APP>/<scope>)");
    }

    result
}

/// Read a fixture file relative to the crate manifest dir.
pub fn read_fixture(rel: &str) -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join(rel);
    std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("failed to read fixture {}: {}", path.display(), e))
}
