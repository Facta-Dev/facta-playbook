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
| `settings.fragment.json` | managed — merged into `settings.json`, never replaces it |

User-owned = the installer skips an existing destination unless `--force`;
managed = the installer writes it. Overwrites are always backed up.

`kit/` is the single source of truth for the user scope: edit here, then
reinstall — do not hand-edit `~/.claude`. No secrets and no `@`-imports of
playbooks in this directory; `settings.fragment.json` carries no endpoint or
token (see `CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md` §3).
