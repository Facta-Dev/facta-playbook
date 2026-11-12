# ADR 0002 — One static library per layer

| | |
| --- | --- |
| **Status** | Accepted |
| **Date** | 2026-11-12 |
| **Scope** | The build structure of the desktop template |
| **Supersedes** | — |

## Context

The template's architecture is a set of rules about which layer may use
which. Those rules are only worth something if a violation is **caught by
the machine**, not by a reviewer who has to know the rules.

A single library with every source in it cannot express the rule: if the
domain and the widgets are in the same binary, the domain can call a widget
and the compiler is happy. The rule survives only as a convention, and
conventions decay.

So the build structure is not a packaging preference. It is the enforcement
mechanism.

## Decision

**Every layer is its own static library, and the dependency rules are
written as `target_link_libraries` edges.** A layer that uses a symbol it
was not allowed to use fails at **link time**, with an undefined reference.

The targets and the rules, exactly as the root
[`CMakeLists.txt`](../../CMakeLists.txt) declares them:

| Target | Links | Must not use |
| --- | --- | --- |
| `desktop_shared_kernel` | Qt Core | Everything else |
| `desktop_contracts` | kernel | Widgets, SQL, HTTP |
| `desktop_domain` | kernel, contracts | Widgets, SQL, HTTP, the OS |
| `desktop_application` | domain | Adapters, Widgets, SQL |
| `desktop_platform` | kernel | The domain, the application layer |
| `desktop_infrastructure` | application, platform, contracts | The view |
| `desktop_ui_kit` | — | The domain, the application layer |
| `desktop_presentation` | application, slice view models, ui_kit | SQL, HTTP, the domain's storage |
| `desktop_sdk` | contracts, Qt Core, Qt Network | Widgets, SQL |
| `desktop_notes_adapters` | application, infrastructure, sdk | The view |
| `desktop_notes_presentation` | application | The view, SQL, HTTP |
| `<app>-desktop` (executable) | presentation, infrastructure, platform, slice adapters | — |

Note the direction: **a library only lists what it may use.** The
application layer does not know that the infrastructure exists; the
infrastructure layer lists the application layer because it implements the
application layer's ports. A dependency that is not declared cannot be
resolved, and the link fails.

## Alternatives considered

### One library, with the rule enforced by review

**Rejected.** It is the status quo ante, and it is exactly what the
architecture suite had to be written to compensate for.

### Header-only layers

**Rejected.** A header-only layer puts the dependency in the *include*, not
in the link, so the check happens in the preprocessor and the failure
message is unreadable. It also forces every translation unit to recompile
when a shared header changes.

### A shared object per layer

**Rejected for now.** It is a stronger boundary, but it adds a runtime
loading step, a path-resolution problem at install time and a plugin
mechanism the product does not need. The link-time rule is the one that
catches the mistake that actually happens.

### A single executable with everything

**Rejected.** Same reason as the single library: no boundary, no
enforcement.

## The three guards

The link rule is the primary one. Two more make it robust.

### 1 · Configure-time include guard

Every layer calls
`desktop_assert_no_forbidden_include(<layer> <dir> <forbidden-regex>)`:

```cmake
function(desktop_assert_no_forbidden_include layer dir forbidden)
  file(GLOB_RECURSE layer_sources CONFIGURE_DEPENDS
       "${dir}/src/*.cpp" "${dir}/src/*.h" "${dir}/include/*.h")
  foreach(layer_source IN LISTS layer_sources)
    file(READ "${layer_source}" layer_content)
    if(layer_content MATCHES "#[ \t]*include[ \t]*[<\"][^\">]*(${forbidden})")
      message(FATAL_ERROR "${layer} must not include headers matching '${forbidden}': ${layer_source}")
    endif()
  endforeach()
endfunction()
```

The failure is a `FATAL_ERROR` at configure time, naming the file and the
pattern. This catches the mistake *before* it becomes a link error, and it
catches it with a message a human can act on.

### 2 · Link time

The rule itself. A call to a symbol the layer did not link produces:

```
undefined reference to `desktop::ui_kit::styleSheet()'
```

No include needed — a forward declaration is enough to produce the call,
and the link is where the missing library shows up.

### 3 · Test time

`tests/_platform/architecture/tst_architecture.cpp` reads the source tree
and checks three rules that a link rule cannot express:

1. The domain of a slice never includes a QtWidgets header.
2. An OS-specific header is only included inside `src/platform`.
3. The application layer of a slice never includes an infrastructure header.

The rules are read from the sources rather than from a list of the files
that exist today, so a **new** file is checked too.

## Consequences

**Accepted:**

- A dependency violation is a **build failure**, not a review comment. The
  compiler is the enforcer.
- The layers can be reasoned about independently: the domain is a library
  with no GUI, the platform is a library with no domain.
- Each slice's adapter and view model are separate targets, so a slice can
  be replaced without touching the others.
- The tests link only what they need: a domain test never pulls in
  Widgets.

**Accepted, with a known cost:**

- The number of targets grows with the number of slices. Each new slice
  adds two targets (adapters, presentation) and one link-rule block. The
  `desktop_add_placeholder_library()` helper in the root `CMakeLists.txt`
  exists for a layer that has no source yet (no layer calls it today).
- The shipped slice names its domain and application targets
  `desktop_domain` and `desktop_application` — **without the slice name**.
  A second slice therefore has to rename them (`desktop_<name>_domain`,
  `desktop_<name>_application`); two slices cannot both claim the same
  target name.
- Static libraries mean the linker may drop a translation unit it thinks is
  unused. This is why the tests link the layer libraries explicitly rather
  than relying on the application target to pull them in.
- The rule is written in CMake, and a reader has to look at the
  `target_link_libraries` block to know the rule. The table above is the
  human-readable copy, and it must be kept in sync — the
  [architecture suite](../../tests/_platform/architecture/tst_architecture.cpp)
  is what keeps the sync honest for the rules it covers.

**Consequences for a new slice:**

```cmake
add_library(desktop_<name>_adapters STATIC
    src/features/<name>/adapters/src/<file>.cpp)
target_link_libraries(desktop_<name>_adapters PUBLIC
    desktop_application desktop_infrastructure desktop_sdk)
```

and the same for the presentation target. The rules are copied, not
invented: an adapter may use the application layer, the infrastructure and
the SDK; a view model may use the application layer and nothing else.

## See also

- [04 — Hexagonal Layers & MVVM](../04-hexagonal-layers-and-mvvm.md) — the
  layers and the guards.
- [05 — Vertical Slicing](../05-vertical-slicing.md) — the per-slice
  targets.
- [12 — Testing & CI](../12-testing-and-ci.md) — the architecture suite.
- [`CMakeLists.txt`](../../CMakeLists.txt) — the rules in their source form.
