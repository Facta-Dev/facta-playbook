# kit/ — canonical user-scope Claude Code files

`kit/` holds the user-scope Claude Code layer (the `~/.claude` layer) as
versioned files. `scripts/claude-kit/install-user-kit.sh` copies it into
`~/.claude` on a machine.

| Path | Ownership |
| --- | --- |
| `CLAUDE.md` | user-owned content, repo-canonical |
| `agents/*.md` | user-owned content, repo-canonical (`advisor`, `kit-curator`) |
| `skills/*/SKILL.md` | user-owned content, repo-canonical (8 SDD + audit skills) |
| `templates/sdd/*.md` | user-owned content, repo-canonical |
| `settings.fragment.json` | managed — base fragment (default model + deny list), merged into `settings.json` on every machine |
| `settings.ollama.json` | managed — Ollama-only fragment (tier aliases, endpoint, subagent model), merged when Ollama is the provider |

User-owned = the installer skips an existing destination unless `--force`;
managed = the installer writes it. Overwrites are always backed up.

`kit/` is the single source of truth for the user scope: edit here, then
reinstall — do not hand-edit `~/.claude`. No secrets and no `@`-imports of
playbooks in this directory. `settings.ollama.json` carries the Ollama
endpoint and the fixed literal token `ollama` (not a secret), so a bare
`claude` reaches Ollama on a fresh machine; the installer merges it only when
a local Ollama endpoint answers (`--provider=` overrides). See
`CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md` §3.

`kit/CLAUDE.md` does not import `@RTK.md`: RTK is optional machine tooling.
On a machine that uses it, keep (or add) `@RTK.md` as the first line of
`~/.claude/CLAUDE.md`; the installer skips an existing file unless `--force`.
