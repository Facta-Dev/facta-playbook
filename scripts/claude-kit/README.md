# scripts/claude-kit — the install layer

Shell-only, zero model tokens. Run with `bash`; no dependencies beyond git,
python3 and the standard shell. Every script takes `--flag=value` or `--flag value`.

| Script | Flags | Exit |
| --- | --- | --- |
| `bootstrap.sh` | `--provider=ollama\|anthropic\|auto` (default auto), `--group=`, `--dir=DIR`, `--dry-run` | 0 ok, 1 precondition missing or a step failed |
| `install-user-kit.sh` | `--dry-run`, `--force`, `--no-settings`, `--provider=ollama\|anthropic\|auto`, `--dest=DIR` | 0 ok, 1 error |
| `clone-repos.sh` | `--group=selected\|core\|products\|tools\|clients\|reference\|all` (default `selected`), `--dir=DIR`, `--depth=N`, `--dry-run` | 0 cloned/skipped, 1 a clone failed |
| `install-skill-packs.sh` | `--dir=DIR`, `--local`, `--remote`, `--only=NAME[,NAME]`, `--dry-run`, `--self-test` | 0 installed/skipped, 1 an install failed |
| `configure-repo.sh` | `<repo-path>`, `--kit=golang\|nodejs\|python\|rust\|extension\|desktop\|frontend-mobile\|mobile-android`, `--check`, `--dry-run` | 0 ok, 1 drift or error |
| `kit_check.py` | `--root=DIR`, `--json`, `--self-test` | 0 clean, 1 warnings, 2 errors |
| `dedupe_plugins.py` | `--root=DIR`, `--backup-dir=DIR`, `--apply`, `--restore`, `--rebuild-manifest` | 0 always; failures are reported in the output |

## The contract

- **Never delete.** No script removes a file it did not create.
- **Never overwrite a user-owned file.** An existing destination is skipped,
  not replaced; `--force` applies only to installer-owned files.
- **Back up before writing.** A changed `settings.json` is copied to
  `~/.claude/backups/install-<UTC timestamp>/` (user scope) or
  `.claude/backups/configure-<YYYY-MM-DD>/` (repo scope).
- **Idempotent.** A second run reports "skipped" and changes nothing.
- **No `@-imports`.** `CLAUDE.md` links the playbooks by path or URL.
- **Plugins are registered, never copied.** `install-skill-packs.sh` adds each
  marketplace and installs its plugins through `claude plugin`; nothing lands
  in `~/.claude/skills/`.

See [PLAYBOOK_INSTALL.md](../../PLAYBOOK_INSTALL.md) for the full bootstrap.
