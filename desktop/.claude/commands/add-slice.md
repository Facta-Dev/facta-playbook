---
name: add-slice
description: "Delegate to the add-slice skill — scaffold a new vertical slice in the <app> desktop tree: domain, application, adapters, presentation, tests, and the CMake targets that keep the layer law."
division: core
---

# /add-slice

## Delegate

Delegate to the **add-slice** skill. Activate when a `<app>` feature needs its
own vertical slice under `src/features/<slice>/`, with its four layers, its
ports, its tests and its wiring in the composition root.
