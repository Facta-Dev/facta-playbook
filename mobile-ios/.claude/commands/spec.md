---
name: spec
description: "Delegate to the spec skill — write the behaviour spec for a <app> iOS change: acceptance criteria with IDs, non-goals, route and capability surface, and open questions, before any implementation."
division: core
---

# /spec

## Delegate

Delegate to the **spec** skill. Activate when a feature, a behaviour change or
a platform capability needs a shared definition of done.

Steps:

1. Write the behaviour as what the user observes and what the app guarantees,
   not as an implementation sketch.
2. Write each acceptance criterion as `AC-N <input> -> <observable result>`,
   testable and citable by a test name.
3. Write the non-goals; the scope the request does not include is binding.
4. Name the surface: routes, deep links, `Info.plist` keys, entitlements,
   push categories and permissions added or changed — or "none" explicitly.
5. Name the `UiStatus` coverage of every new screen, including error and empty
   states.
6. List open questions and mark each blocking or answered; a silent assumption
   is a rejected spec.
7. Write the spec to `docs/specs/<slug>.md` and hand it to `review` or the
   human before `plan` starts.
