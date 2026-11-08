# Claude Code model routing on Ollama — portable runbook

How this machine (and any other one) runs Claude Code with three model tiers
and an on-demand advisor, using only Ollama cloud models. Verified on Claude
Code 2.1.319 + Ollama 0.33.3, November 8, 2026.

The goal is the advisor pattern: a cheap model writes, a cheaper one digs, and
a strong one is consulted only before a plan locks, after the same error twice,
and before a task is called done. **The native `--advisor` flag does not work
on Ollama** (see §7); this runbook reproduces the pattern with a subagent.

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

Never store API keys in this file; it is plain text.

## 4. `~/.claude/agents/advisor.md`

```markdown
---
name: advisor
description: Asesor de decisión. Consultar antes de fijar un plan, cuando el mismo error reaparezca dos veces, y antes de declarar una tarea terminada. Solo lee y decide.
tools: Read, Grep, Glob
model: opus
---

Sos el asesor de decisión. Recibís el contexto que te pasa la sesión principal
(plan, diff, resultados de tests, transcript parcial).

Respondé SOLO:

1. Riesgo no obvio que el trabajo actual no está cubriendo.
2. Decisión recomendada, en una frase imperativa.
3. Evidencia que falta antes de avanzar (o "suficiente" si no falta nada).

Reglas:

- No edites archivos ni ejecutes comandos: tu salida es consejo.
- Si el contexto es insuficiente para decidir, decilo y pedí exactamente qué leer.
- No repitas el plan de vuelta ni resumas el diff; el valor está en lo que falta.
- Preferí la solución más simple que resuelve el problema real; marcá el
  over-engineering cuando lo veas.
```

`model: opus` is the alias, so moving the escalation model is one env change,
not an edit here. Since v2.1.251 this frontmatter value overrides
`CLAUDE_CODE_SUBAGENT_MODEL`; `CLAUDE_CODE_SUBAGENT_MODEL_FORCE=1` would
override it in turn.

## 5. `~/.claude/CLAUDE.md`

```markdown
## Model tiers (Ollama)

Los alias resuelven por `ANTHROPIC_DEFAULT_*` (settings.json): `opus` =
escalation, `sonnet` = workhorse, `haiku` = barato. Subí de tier con
`/model opus` cuando el problema es de diseño o la misma solución falló dos
veces; volvé a `sonnet` al terminar.

- Consultá el agent `advisor` antes de fijar un plan, cuando el mismo error
  reaparezca dos veces, y antes de declarar una tarea terminada.
- Pasale contexto concreto (plan, diff, output del test); no le pidas que
  investigue el repo entero.
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
claude -p "Invocá el subagent advisor con este contexto: 'test'. Devolvé solo su respuesta." \
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

Remove the five `env` keys and `"model": "sonnet"` from `settings.json`, delete
`~/.claude/agents/advisor.md`, and revert the `~/.claude/CLAUDE.md` section.
Per-repo blocks are inert without the global settings.
