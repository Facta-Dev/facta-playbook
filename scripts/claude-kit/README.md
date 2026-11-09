# scripts/claude-kit — the install layer

Shell-only, zero model tokens. Run with `bash`; no dependencies beyond git,
python3 and the standard shell. Every script takes `--flag=value` or `--flag value`.

| Script | Flags | Exit |
| --- | --- | --- |
| `install-user-kit.sh` | `--dry-run`, `--force`, `--no-settings`, `--dest=DIR` | 0 ok, 1 error |
| `clone-repos.sh` | `--group=core\|products\|tools\|clients\|reference\|all`, `--dir=DIR`, `--depth=N`, `--dry-run` | 0 cloned/skipped, 1 a clone failed |
| `configure-repo.sh` | `<repo-path>`, `--kit=golang\|nodejs\|python\|rust`, `--check`, `--dry-run` | 0 ok, 1 drift or error |
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

See [PLAYBOOK_INSTALL.md](../../PLAYBOOK_INSTALL.md) for the full bootstrap.
