// contracts_test.go — pure-std contract validator + tests for the .claude
// layer.
//
// Hand-rolled frontmatter parser + body-section checker for agents, skills,
// and commands. Zero external dependencies: no yaml crate equivalent, no
// markdown parser — std only. The required fields, required sections,
// allowed divisions, and failure modes are HARDCODED as static slices in
// requiredFrontmatter() / requiredSections() / allowedDivisions() below —
// this validator does NOT read .claude/schemas/*.schema.md at runtime. The
// schema docs are the source of truth and the bash linters
// (scripts/lint-skills.sh, scripts/lint-commands.sh, scripts/lint-agents.sh)
// parse them at runtime, so the linter<->schema drift guarantee holds via
// the linters. This validator is a hand-maintained mirror of the schema
// docs for the native `go test` runner: when a list changes in a schema
// doc you MUST update the matching static slice here too.
//
// The validator is the contract surface exercised by the tests below over
// fixtures/valid/*.md and fixtures/invalid/*.md. Each invalid fixture
// exercises a distinct failure mode; the test asserts the validator rejects
// it and that the joined error message contains a known substring
// (unanchored, mirroring assert.Contains). The fixtures are FROZEN test
// data: rename.sh excludes evals/fixtures/ from its substitution pass, so
// the invalid leak fixture keeps its literal tokens post-rename and keeps
// exercising the leak detector.
//
// This file carries NO literal template substitution token anywhere
// (rename.sh
// rewrites evals/*.go): the leak-detector tokens are assembled at runtime in
// placeholderTokens and the leak error message interpolates them. A literal
// token here would be substituted away on rename and silently defuse the
// detector — the fixture-based test below fails LOUDLY in that case, but
// staying placeholder-free keeps the validator byte-identical across rename.
package evals

import (
	"bytes"
	"fmt"
	"os"
	"path/filepath"
	"runtime"
	"slices"
	"strings"
	"testing"
)

// Which .claude module kind a document belongs to.
type kind int

const (
	kindAgent kind = iota
	kindSkill
	kindCommand
)

// placeholderTokens — the three template substitution tokens, assembled from
// pieces so this file itself never spells one out literally: rename.sh
// rewrites evals/*.go during instantiation, and a literal token in this file
// would be substituted away, breaking the leak detector AND the sweep below.
var placeholderTokens = []string{"<" + "app>", "<" + "APP>", "<" + "scope>"}

func (k kind) requiredFrontmatter() []string {
	switch k {
	case kindAgent:
		// Matches agents.schema.md "Required Frontmatter Fields".
		return []string{"name", "description", "color", "division"}
	default: // skill, command
		// Matches skills/commands schema "Required Frontmatter Fields".
		return []string{"name", "description", "division"}
	}
}

func (k kind) requiredSections() []string {
	switch k {
	case kindAgent: // agents.schema.md "Required Body Sections"
		return []string{
			"Core Mission",
			"Expertise Areas",
			"Output Format",
			"Quality Standards",
			"Guiding Principles",
		}
	case kindSkill: // skills.schema.md "Required Body Sections"
		return []string{
			"When to Activate",
			"Core Principles",
			"Rules",
			"Research References",
			"Output Format",
		}
	default: // command
		return []string{"Delegate"}
	}
}

func (k kind) allowedDivisions() []string {
	// Shared across agents/skills/commands in this template.
	return []string{"backend", "security", "testing", "core"}
}

// result of validating one document.
type result struct {
	ok   bool
	errs []string
}

func (r *result) fail(msg string) {
	r.ok = false
	r.errs = append(r.errs, msg)
}

func (r result) joined() string { return strings.Join(r.errs, "\n") }

type kv struct {
	key, val string
}

// parseFrontmatter parses a YAML frontmatter block into an ordered slice of
// key/value pairs plus the body after the closing fence.
//
// Accepts the leading "---\n" fence, reads until the closing "---", and
// parses "key: value" lines (stripping one layer of surrounding single or
// double quotes from the value). Returns closed=false when the frontmatter
// is missing or malformed.
func parseFrontmatter(text string) (fm []kv, body string, closed bool) {
	lines := strings.Split(text, "\n")
	if len(lines) == 0 || strings.TrimSpace(lines[0]) != "---" {
		return nil, text, false
	}
	var bodyLines []string
	for _, line := range lines[1:] {
		if !closed {
			if strings.TrimSpace(line) == "---" {
				closed = true
				continue
			}
			// Skip blank lines and comments inside frontmatter.
			t := strings.TrimSpace(line)
			if t == "" || strings.HasPrefix(t, "#") {
				continue
			}
			if before, after, found := strings.Cut(line, ":"); found {
				key := strings.TrimSpace(before)
				val := strings.TrimSpace(after)
				// Strip one layer of matching surrounding quotes.
				if len(val) >= 2 {
					first, last := val[0], val[len(val)-1]
					if (first == '"' || first == '\'') && first == last {
						val = val[1 : len(val)-1]
					}
				}
				fm = append(fm, kv{key, val})
			}
			continue
		}
		bodyLines = append(bodyLines, line)
	}
	return fm, strings.Join(bodyLines, "\n"), closed
}

// fmHas reports whether the key is present in the parsed frontmatter.
func fmHas(fm []kv, key string) bool {
	return slices.ContainsFunc(fm, func(e kv) bool { return e.key == key })
}

// fmGet looks up a frontmatter value by key (first match; "" when absent).
func fmGet(fm []kv, key string) string {
	for _, e := range fm {
		if e.key == key {
			return e.val
		}
	}
	return ""
}

// hasSection checks whether the body contains a heading matching section
// (case-insensitive), accepting one or more # prefixes.
func hasSection(body, section string) bool {
	needle := strings.ToLower(section)
	for _, line := range strings.Split(body, "\n") {
		t := strings.TrimLeft(line, " \t")
		if !strings.HasPrefix(t, "#") {
			continue
		}
		heading := strings.TrimLeft(t[1:], "#")
		if strings.EqualFold(strings.TrimSpace(heading), needle) {
			return true
		}
	}
	return false
}

// hasPlaceholderLeak checks for residual literal placeholders from the
// rename pass — the three tokens assembled in placeholderTokens.
func hasPlaceholderLeak(text string) bool {
	for _, tok := range placeholderTokens {
		if strings.Contains(text, tok) {
			return true
		}
	}
	return false
}

// validate checks a single document against the rules for kind.
func validate(text string, k kind) result {
	r := result{ok: true}

	// CRLF check.
	if strings.ContainsRune(text, '\r') {
		r.fail("CRLF line endings — convert to LF")
	}

	fm, body, closed := parseFrontmatter(text)
	if !closed {
		r.fail("missing or malformed frontmatter (no closing ---)")
		return r
	}
	if fm == nil {
		r.fail("empty frontmatter")
	}

	// Required frontmatter fields.
	for _, field := range k.requiredFrontmatter() {
		if !fmHas(fm, field) {
			r.fail(fmt.Sprintf("missing frontmatter key '%s'", field))
		}
	}

	// Division membership in the allowed enum.
	if fmHas(fm, "division") {
		if div := fmGet(fm, "division"); !slices.Contains(k.allowedDivisions(), div) {
			r.fail(fmt.Sprintf("bad division '%s'", div))
		}
	}

	// Required body sections.
	for _, section := range k.requiredSections() {
		if !hasSection(body, section) {
			r.fail(fmt.Sprintf("missing body section '%s'", section))
		}
	}

	// Literal placeholder leak (post-rename contract: 0 residual).
	if hasPlaceholderLeak(text) {
		r.fail(fmt.Sprintf(
			"literal placeholder leak (%s) — rename.sh must substitute all placeholders",
			strings.Join(placeholderTokens, "/")))
	}

	return r
}

// domainImportViolations scans the import block(s) of a domain.go text and
// returns one violation per forbidden import. The domain core must import
// stdlib + internal/shared only — no adapter layer (net/http, database/sql,
// pgx), no sibling slice, no otel (observed CUT from this template), and no
// unsafe. Mirrors the depguard deny rules in .golangci.yml that
// scripts/lint-agents.sh's agents contract assumes; kept hand-maintained so
// this file needs no .golangci.yml parse.
func domainImportViolations(text string) []string {
	var paths []string
	lines := strings.Split(text, "\n")
	inBlock := false
	for _, line := range lines {
		t := strings.TrimSpace(line)
		switch {
		case inBlock:
			if t == ")" {
				inBlock = false
				continue
			}
			if p, quoted := cutImport(stripLineComment(t)); quoted {
				paths = append(paths, p)
			}
		case strings.HasPrefix(t, "import") && strings.HasSuffix(t, "("):
			inBlock = true
		default:
			// Single-line import "path" (with optional alias prefix).
			if _, rest, found := strings.Cut(t, "import"); found {
				if p, quoted := cutImport(stripLineComment(rest)); quoted {
					paths = append(paths, p)
				}
			}
		}
	}

	var violations []string
	for _, p := range paths {
		switch {
		case p == "net/http":
			violations = append(violations,
				"forbidden import 'net/http' — domain.go is pure (stdlib + internal/shared only); move it to the slice's http.go")
		case p == "database/sql":
			violations = append(violations,
				"forbidden import 'database/sql' — domain.go is pure (stdlib + internal/shared only); move it to the slice's postgres.go")
		case strings.Contains(p, "jackc/pgx"):
			violations = append(violations,
				fmt.Sprintf("forbidden import '%s' — the pgx driver belongs to the slice's postgres.go adapter", p))
		case p == "unsafe":
			violations = append(violations,
				"forbidden import 'unsafe' — no unsafe in the internal tree")
		case strings.Contains(p, "go.opentelemetry.io"):
			violations = append(violations,
				fmt.Sprintf("forbidden import '%s' — otel is CUT from this template (observability is slog + /healthz + /readyz)", p))
		case strings.Contains(p, "internal/features/"):
			violations = append(violations,
				fmt.Sprintf("cross-slice import '%s' — slices never import each other; route shared shapes through internal/shared", p))
		}
	}
	return violations
}

// stripLineComment removes a trailing // comment from an import clause.
func stripLineComment(clause string) string {
	if i := strings.Index(clause, "//"); i >= 0 {
		return clause[:i]
	}
	return clause
}

// cutImport strips a leading alias ("x " or `x `) from an import clause and
// returns the quoted path when the clause ends in one.
func cutImport(clause string) (path string, ok bool) {
	clause = strings.TrimSpace(clause)
	if i := strings.LastIndex(clause, " "); i >= 0 {
		clause = clause[i+1:]
	}
	if len(clause) >= 2 && clause[0] == '"' && clause[len(clause)-1] == '"' {
		return clause[1 : len(clause)-1], true
	}
	return "", false
}

// evalsDir resolves this test file's directory at compile time, so fixture
// reads work from any working directory (and inside a staged/renamed copy,
// where go test compiles the copied tree in place).
func evalsDir() string {
	_, file, _, ok := runtime.Caller(0)
	if !ok {
		panic("cannot resolve contracts_test.go path")
	}
	return filepath.Dir(file)
}

func readFixture(t *testing.T, rel string) string {
	t.Helper()
	data, err := os.ReadFile(filepath.Join(evalsDir(), filepath.FromSlash(rel)))
	if err != nil {
		t.Fatalf("failed to read fixture %s: %v", rel, err)
	}
	return string(data)
}

// repoRoot is the directory containing evals/ (the skeleton root) — the
// parent of this test file's directory.
func repoRoot(t *testing.T) string {
	t.Helper()
	return filepath.Dir(evalsDir())
}

// renamedReported reports whether the tree this test binary was compiled in
// has already been through scripts/rename.sh (its evals/go.mod carries the
// substituted module path). Pre-rename the tree legitimately carries
// placeholder vocabulary everywhere outside the agency layer, so the sweep
// below skips itself in that state instead of failing.
func renamedReported(t *testing.T) bool {
	t.Helper()
	mod, err := os.ReadFile(filepath.Join(evalsDir(), "go.mod"))
	if err != nil {
		t.Fatalf("failed to read evals/go.mod: %v", err)
	}
	return !hasPlaceholderLeak(string(mod))
}

func assertPass(t *testing.T, r result, ctx string) {
	t.Helper()
	if !r.ok {
		t.Fatalf("%s: expected ok, got errors:\n%s", ctx, r.joined())
	}
}

func assertFailMatching(t *testing.T, r result, pat, ctx string) {
	t.Helper()
	if r.ok {
		t.Fatalf("%s: expected failure, but validator passed", ctx)
	}
	if !strings.Contains(r.joined(), pat) {
		t.Fatalf("%s: expected errors to contain '%s', got:\n%s", ctx, pat, r.joined())
	}
}

func TestValidSkillFixturePasses(t *testing.T) {
	r := validate(readFixture(t, "fixtures/valid/skill.md"), kindSkill)
	assertPass(t, r, "valid skill")
}

func TestValidAgentFixturePasses(t *testing.T) {
	r := validate(readFixture(t, "fixtures/valid/agent.md"), kindAgent)
	assertPass(t, r, "valid agent")
}

func TestValidCommandFixturePasses(t *testing.T) {
	r := validate(readFixture(t, "fixtures/valid/command.md"), kindCommand)
	assertPass(t, r, "valid command")
}

func TestInvalidMissingFrontmatterKeyFails(t *testing.T) {
	r := validate(readFixture(t, "fixtures/invalid/missing-frontmatter-key.md"), kindSkill)
	assertFailMatching(t, r, "missing frontmatter key 'division'", "missing-frontmatter-key")
}

func TestInvalidMissingBodySectionFails(t *testing.T) {
	r := validate(readFixture(t, "fixtures/invalid/missing-body-section.md"), kindSkill)
	assertFailMatching(t, r, "missing body section 'Output Format'", "missing-body-section")
}

func TestInvalidLiteralPlaceholderLeakFails(t *testing.T) {
	r := validate(readFixture(t, "fixtures/invalid/literal-placeholder-leak.md"), kindSkill)
	assertFailMatching(t, r, "literal placeholder leak", "literal-placeholder-leak")
}

func TestInvalidDomainImportsHTTPFails(t *testing.T) {
	violations := domainImportViolations(readFixture(t, "fixtures/invalid/bad-slice-import.go.txt"))
	joined := strings.Join(violations, "\n")
	if len(violations) == 0 {
		t.Fatalf("bad-slice-import: expected violations, got none")
	}
	if !strings.Contains(joined, "forbidden import 'net/http'") {
		t.Fatalf("bad-slice-import: wanted a net/http violation, got:\n%s", joined)
	}
}

func TestBadDivisionFails(t *testing.T) {
	text := "---\nname: \"s\"\ndescription: \"d\"\ndivision: frontend\n---\n\n## When to Activate\n\nx"
	r := validate(text, kindSkill)
	assertFailMatching(t, r, "bad division 'frontend'", "bad-division")
}

func TestEmptyFrontmatterFails(t *testing.T) {
	r := validate("---\n---\n\n# body\n", kindSkill)
	assertFailMatching(t, r, "empty frontmatter", "empty-frontmatter")
}

func TestNoFrontmatterFails(t *testing.T) {
	r := validate("# No frontmatter here\n\n## When to Activate\n", kindSkill)
	assertFailMatching(t, r, "missing or malformed frontmatter", "no-frontmatter")
}

func TestCRLFLineEndingsFail(t *testing.T) {
	text := strings.ReplaceAll(readFixture(t, "fixtures/valid/skill.md"), "\n", "\r\n")
	r := validate(text, kindSkill)
	assertFailMatching(t, r, "CRLF line endings", "crlf")
}

func TestPureDomainImportsPass(t *testing.T) {
	// A pure domain.go: stdlib + kernel only. The module path is the inert
	// post-rename display vocabulary (frozen fixture style), never one of
	// the substitution tokens — see placeholderTokens.
	text := `package widgets

import (
	"strings"

	"github.com/acme/demoapp/internal/shared/clockx"
)

type Widget struct {
	ID        string
	CreatedAt clockx.Time
}

func Slug(name string) string { return strings.ToLower(name) }
`
	if v := domainImportViolations(text); len(v) != 0 {
		t.Fatalf("pure domain.go flagged: %s", strings.Join(v, "\n"))
	}
}

func TestCrossSliceImportFlagged(t *testing.T) {
	text := `package widgets

import "github.com/acme/demoapp/internal/features/authsignup"
`
	if v := domainImportViolations(text); len(v) == 0 {
		t.Fatalf("cross-slice import not flagged")
	}
}

// TestNoPlaceholderLeaksOutsideAgencyLayer sweeps the tree (post-rename) for
// residual literal placeholders OUTSIDE the agency layer. scripts/ keeps its
// substitution-token pattern strings by design (the gates carry their own
// grep patterns), evals/ is skipped because this test file assembles its
// tokens at runtime and fixtures/ is frozen leak-detector data, and .git +
// binaries + go.sum are not text. Mirrors scripts/leak-guard.sh's exclusion
// semantics; anything else containing one of the tokens is a rename bug.
// verify-template.sh's own survivor grep (which also excludes .claude) is
// the second, slightly weaker copy of this check.
func TestNoPlaceholderLeaksOutsideAgencyLayer(t *testing.T) {
	if !renamedReported(t) {
		t.Skip("running pre-rename: the tree legitimately carries placeholder vocabulary outside the agency layer; verify-template.sh (rename, then test) exercises this check post-rename")
	}
	root := repoRoot(t)

	skipDirs := map[string]bool{
		".git":         true,
		"scripts":      true,
		"evals":        true,
		"fixtures":     true,
		"node_modules": true,
		"bin":          true,
		"target":       true,
	}
	binaryExt := map[string]bool{
		".png": true, ".jpg": true, ".jpeg": true, ".ico": true, ".gif": true,
		".woff": true, ".woff2": true, ".ttf": true, ".otf": true, ".eot": true,
	}

	var leaks []string
	err := filepath.WalkDir(root, func(path string, d os.DirEntry, err error) error {
		if err != nil {
			return err
		}
		if d.IsDir() {
			if path == root {
				return nil
			}
			if skipDirs[d.Name()] {
				return filepath.SkipDir
			}
			return nil
		}
		base := d.Name()
		if strings.HasSuffix(base, ".sum") || binaryExt[strings.ToLower(filepath.Ext(base))] {
			return nil
		}
		data, err := os.ReadFile(path)
		if err != nil {
			return err
		}
		if bytes.IndexByte(data, 0) >= 0 {
			return nil // binary; leak-guard greps text files only
		}
		if hasPlaceholderLeak(string(data)) {
			rel, rerr := filepath.Rel(root, path)
			if rerr != nil {
				rel = path
			}
			leaks = append(leaks, rel)
		}
		return nil
	})
	if err != nil {
		t.Fatalf("placeholder sweep walk failed: %v", err)
	}
	if len(leaks) > 0 {
		t.Fatalf("literal placeholder leak outside the agency layer (rename.sh must substitute all of:\n%s\n) in:\n%s",
			strings.Join(placeholderTokens, "/"), strings.Join(leaks, "\n"))
	}
}
