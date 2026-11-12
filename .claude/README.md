# .claude — this repo's Claude Code kit

The playbooks live in the playbook repo, not here. Read them there; do not
restate or import them into this file.

| Playbook | Governs |
| --- | --- |
| [PLAYBOOK_CLAUDE_CODE.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/PLAYBOOK_CLAUDE_CODE.md) | the kit this repo must have |
| [PLAYBOOK_TESTS.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/PLAYBOOK_TESTS.md) | testing contract and gates |
| [PLAYBOOK_SECURITY.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/PLAYBOOK_SECURITY.md) | security contract, per-language profiles |

Runbook: [CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md](https://github.com/Facta-Dev/ai-coding-playbook/blob/main/CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md)
— model tiers, the routing block, `~/.claude/settings.json`.

This repo's own kit is deliberately minimal: `settings.json` (deny list) and
this file. The language kits under `golang/`, `nodejs/`, `python/` and `rust/`
carry their own `.claude/` trees. Verify the wiring with:

```bash
bash ~/ai-coding-playbook/scripts/claude-kit/configure-repo.sh <repo> --check
```
