# facta-playbook — agent instructions

<!-- model-routing:ollama -->
## Model routing (Claude Code + Ollama)

Runbook: `facta-playbook/CLAUDE_CODE_ROUTING_MODELS_OLLAMA.md`.

- Tier aliases resolve through `~/.claude/settings.json`: `opus` = escalation
  (`deepseek-v4-pro:cloud`), `sonnet` = workhorse (`glm-5.3-flash:cloud`),
  `haiku` = cheap digging (`minimax-m3.1-flash:cloud`).
- Consult the `advisor` subagent before locking a plan, when the same error
  reappears twice, and before declaring a task done. Pass concrete context
  (plan, diff, test output) — not the whole repo.
- Escalate with `/model opus` for design decisions or after two failed
  attempts; return to `sonnet` when the task is done.
- All code, comments, tests, specs, and documentation are generated in English.
