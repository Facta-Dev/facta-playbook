# Claude Code model routing on Ollama — portable runbook

How this machine (and any other one) runs Claude Code with three model tiers
and an on-demand advisor, using only Ollama cloud models. Verified on Claude
Code 2.1.319 + Ollama 0.33.3, November 8, 2026.

The goal is the advisor pattern: a cheap model writes, a cheaper one digs, and
a strong one is consulted only before a plan locks, after the same error twice,
and before a task is called done. **The native `--advisor` flag does not work
on Ollama** (see §7); this runbook reproduces the pattern with a subagent.

> **Companion guide: [PLAYBOOK_CLAUDE_CODE.md](PLAYBOOK_CLAUDE_CODE.md)** — the
> Claude Code implementation kit (agents, skills, rules, hooks, roles,
> coordination, SDD/TDD). This runbook is its model-routing slice: §3–§5 map to
> the capability plane (agent roster, `CLAUDE.md`) and the control plane
> (`settings.json`); the `advisor` agent is an ordinary roster entry (§6.3) and
> the tier aliases are the concrete values behind that playbook's `model` field
> (§6.2). Read the playbook first if you are building the kit from scratch;
> come back here to wire the models.

---

## 1. Tiers

| Alias | Role | Ollama model | Used by |
| --- | --- | --- | --- |
| `opus` | escalation / advisor | `deepseek-v4-pro:cloud` (alt: `kimi-k3.1-pro:cloud`) | `advisor` agent, `/model opus` |
| `sonnet` | workhorse, main session | `glm-5.3-flash:cloud` (alt: `deepseek-v4.1-flash:cloud`) | default session, agents pinned `model: sonnet` |
| `haiku` | cheap digging | `minimax-m3.1-flash:cloud` | built-in search/explore, `CLAUDE_CODE_SUBAGENT_MODEL` |

All three must declare `completion`/`tools`/`thinking` on Ollama. Check any
model with:

```bash
curl -s http://127.0.0.1:11434/api/show -d '{"model":"deepseek-v4-pro:cloud"}' \
  | python3 -c "import sys,json; print(json.load(sys.stdin)['capabilities'])"
```

Ollama cloud resolves catalog models on demand; `ollama list` only shows what
is already pulled locally.

## 2. Requirements on a new machine

1. `ollama --version` ≥ 0.33 with `ollama launch claude` support.
2. `claude --version` ≥ 2.1.251 (older builds resolve `CLAUDE_CODE_SUBAGENT_MODEL`
   over the agent's frontmatter `model:`).
3. The three models reachable (§1 check).
4. Ollama listening on `127.0.0.1:11434` (or a gateway on `20128` — if the
   profiles under `~/.claude/profiles/` are used, that gateway must be running).

## 3. `~/.claude/settings.json`

Add to `env` (keep any existing keys):

```json
"ANTHROPIC_DEFAULT_OPUS_MODEL": "deepseek-v4-pro:cloud",
"ANTHROPIC_DEFAULT_SONNET_MODEL": "glm-5.3-flash:cloud",
"ANTHROPIC_DEFAULT_HAIKU_MODEL": "minimax-m3.1-flash:cloud",
"CLAUDE_CODE_SUBAGENT_MODEL": "minimax-m3.1-flash:cloud",
"CLAUDE_CODE_AUTO_COMPACT_WINDOW": "1000000"
```

And at the top level of the same file:

```json
"model": "sonnet"
```

The top-level `model` is not optional: with no explicit model, Claude Code
defaults to the **opus** alias, so after tiering every bare `claude` would run
the expensive model.

`settings.json` `env` overrides what `ollama launch claude` injects — verified
by launching a session that exported the old values and reading its `env`.

If nothing in the shell exports the endpoint, a bare `claude` cannot reach
Ollama (`Failed to authenticate: OAuth session expired`). Add the endpoint to
the same `env` block on those machines:

```json
"ANTHROPIC_BASE_URL": "http://127.0.0.1:11434",
"ANTHROPIC_AUTH_TOKEN": "ollama"
```

The token is the fixed literal `ollama` accepted by the local proxy, not a
secret. Verified on this machine: a bare `claude` then resolves
`glm-5.3-flash:cloud` with the Ollama endpoint env removed from the shell.

Never store API keys in this file; it is plain text.

## 4. `~/.claude/agents/advisor.md`

```markdown
---
name: advisor
description: Decision advisor. Consult before locking a plan, when the same error reappears twice, and before declaring a task done. Read-only.
tools: Read, Grep, Glob
model: opus
---

You are the decision advisor. You receive the context passed by the main
session (plan, diff, test results, partial transcript).

Answer ONLY:

1. Non-obvious risk the current work is not covering.
2. Recommended decision, in one imperative sentence.
3. Evidence missing before moving forward (or "enough" if nothing is missing).

Rules:

- Do not edit files or run commands: your output is advice.
- If the context is insufficient to decide, say so and ask exactly what to read.
- Do not repeat the plan back or summarize the diff; the value is in what is missing.
- Prefer the simplest solution that solves the real problem; flag
  over-engineering when you see it.
```

`model: opus` is the alias, so moving the escalation model is one env change,
not an edit here. Since v2.1.251 this frontmatter value overrides
`CLAUDE_CODE_SUBAGENT_MODEL`; `CLAUDE_CODE_SUBAGENT_MODEL_FORCE=1` would
override it in turn.

## 5. `~/.claude/CLAUDE.md`

```markdown
## Model tiers (Ollama)

Aliases resolve through `ANTHROPIC_DEFAULT_*` (settings.json): `opus` =
escalation, `sonnet` = workhorse, `haiku` = cheap. Escalate with `/model opus`
when the problem is design-related or the same solution has failed twice; go
back to `sonnet` when done.

- Consult the `advisor` agent before locking a plan, when the same error
  reappears twice, and before declaring a task done.
- Pass concrete context (plan, diff, test output); do not ask it to investigate
  the whole repo.
- All code, comments, tests, specs, and documentation are generated in English
  (playbook §2.4).
```

## 6. Per-repo block

Each repo's `CLAUDE.md` carries the routing rule so it survives on a machine
without `~/.claude/CLAUDE.md`. The block is idempotent — look for the
`model-routing:ollama` marker before appending again:

```markdown
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
```

Rollout (2026-11-08): heimdall-platform, enyo/enyo-platform, deino, talos,
staxis, everythink_platform, transportation-bank, v0ss, ctx0/nest, ctx0/ctx0.

## 7. Native `--advisor` — Anthropic only

`claude --model sonnet --advisor opus` exists (experimental, docs
`code.claude.com/docs/en/advisor`) but:

- works **only** against the Anthropic API (not Bedrock/Vertex/proxies) and
  needs a supported **main** model (Fable, Opus 4.6+, Sonnet 4.6+, Haiku
  4.5/5.5);
- with an Ollama model it fails: `does not support the advisor tool`;
- it is feature-flagged; `DISABLE_TELEMETRY` disables it.

Equivalent settings key: `"advisorModel": "opus"`, or `/advisor <model>`
inside a session. On this machine the native path only makes sense when the
main session runs on Claude through the gateway or an Anthropic key.

## 8. Running

```bash
# normal session: settings model ("sonnet" alias)
claude

# or let Ollama pin the main model explicitly
ollama launch claude --model glm-5.3-flash:cloud

# escalate for one decision, then come back
# inside the session:  /model opus   ...   /model sonnet
```

## 9. Verification

```bash
# default tier
claude -p "ok" --output-format json | grep -o '"modelUsage":{"[^"]*'
#   -> "modelUsage":{"glm-5.3-flash:cloud

# escalation tier
claude -p "ok" --model opus --output-format json | grep -o '"modelUsage":{"[^"]*'
#   -> "modelUsage":{"deepseek-v4-pro:cloud

# cheap tier
claude -p "ok" --model haiku --output-format json | grep -o '"modelUsage":{"[^"]*'
#   -> "modelUsage":{"minimax-m3.1-flash:cloud

# advisor agent is discoverable and runs
claude -p "Invoke the advisor subagent with this context: 'test'. Return only its response." \
  --output-format json | grep -o '"subagent_stats":{[^}]*'
#   -> "subagent_stats":{"spawned":1,...
```

## 10. Gotchas

- **Default model = opus alias.** A bare `claude` with no `model` setting runs
  the escalation model; a one-turn probe cost $0.097 on `deepseek-v4-pro`.
  Keep `"model": "sonnet"` in settings.
- **`ollama launch` sets all three aliases** to `--model` and exports
  `ANTHROPIC_DEFAULT_*`; `settings.json` still wins for the keys it defines.
- **`[unrecognized_model]` warnings** for Ollama IDs are benign — Claude Code
  has no metadata for them, assumes a 200k context, and the session runs.
- **Model precedence**: session `/model` > `--model` > `ANTHROPIC_MODEL` >
  settings `model`. `ANTHROPIC_DEFAULT_*` only defines alias resolution.
- **Subagents**: invocation param > frontmatter `model:` >
  `CLAUDE_CODE_SUBAGENT_MODEL` > conversation model.
- **`--fallback-model`** is unrelated to the advisor: it is error/overload
  fallback for the current turn.
- Gateway profiles (`~/.claude/profiles/*`, port `20128`) fail silently if the
  gateway is not running; only one route can be live at a time.

## 11. Rollback

Remove the five tier `env` keys (plus `ANTHROPIC_BASE_URL` /
`ANTHROPIC_AUTH_TOKEN` if you added them) and `"model": "sonnet"` from
`settings.json`, delete
`~/.claude/agents/advisor.md`, and revert the `~/.claude/CLAUDE.md` section.
Per-repo blocks are inert without the global settings.
