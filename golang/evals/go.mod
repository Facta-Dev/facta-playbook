// evals — native contract tests for the template's .claude layer.
//
// Standalone Go module (the parent module's ./... never descends into it
// because verify-template.sh runs `go test ./...` inside this directory
// explicitly). Zero external dependencies: the frontmatter parser and
// body-section checker are hand-rolled on std only, so `go test` runs
// offline with no network. No go.sum is needed (no requires block).
//
// The module path carries the <scope>/<app> placeholders and IS rewritten
// by rename.sh. The validator source (contracts_test.go) carries NO literal
// placeholder tokens by construction: its leak-detector tokens are assembled
// at runtime ("<" + "app>") so the substitution pass cannot defuse the leak
// detector (see placeholderTokens there). evals/fixtures/ is excluded from
// rename and stays literal.
module github.com/<scope>/<app>/evals

go 1.27