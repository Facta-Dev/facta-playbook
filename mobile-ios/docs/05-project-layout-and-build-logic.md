# Project Layout & Build Logic

The tree, the manifests, the version pins and the command that proves it all
compiles.

## 1. The tree

```text
mobile-ios/
├── project.yml                  # the XcodeGen manifest — the source of truth for the project
├── Makefile                     # make help/generate/build/test/evals/lint/clean/verify/ci
├── .gitignore                   # ignores *.xcodeproj/, DerivedData/, .build/, .swiftpm/, Package.resolved
├── Apps/<APP>/Sources/          # the app target: @main, AppEnvironment, Router, RootView
│   ├── <APP>App.swift           # the @main App — renamed to <NEW_APP>App.swift by the file pass
│   ├── AppEnvironment.swift     # the composition root: builds the object graph
│   ├── Router.swift             # the typed navigation path
│   └── RootView.swift           # NavigationStack + navigationDestination
├── Packages/
│   ├── Core/
│   │   ├── Kernel/              # pure Swift: Outcome, AppError, ErrorCode, NoteID
│   │   └── UICore/              # pure Swift: UiStatus, the closed status set
│   └── Feature/Notes/           # the example slice — replace with your domain
│       ├── Contracts/           #   pure Swift: NotesRoute, the typed routes
│       ├── Domain/              #   pure Swift: Note, NoteRepository, LoadNotes
│       ├── Data/                #   the in-memory adapter
│       └── UI/                  #   SwiftUI screens + @Observable model
├── evals/                       # the .claude contract tests (node --test) + the live tier
├── scripts/
│   ├── check-architecture.sh    # the architecture gate
│   ├── rename.sh                # the three-token substitution
│   ├── native-gates.sh          # swift test per package + xcodegen + xcodebuild
│   ├── verify-template.sh       # runs every gate on a renamed copy under /tmp
│   ├── install-claude-runtime.sh # installs the kit's hooks and settings
│   ├── instruction-budget.sh    # the instruction budget of the kit's own files
│   ├── leak-guard.sh            # the placeholder and secret sweep
│   ├── lint-agents.sh           # the agents linter
│   ├── lint-commands.sh         # the commands linter
│   ├── lint-skills.sh           # the skills linter
│   └── hooks/guard-bash.sh      # the PreToolUse guard
├── .claude/                     # the agent kit: agents, skills, commands, schemas
├── docs/                        # this set
├── AGENTS.md                    # the kit's agent instructions
├── MEMORY.md                    # the kit's memory layer
├── CLAUDE.md -> AGENTS.md       # the symlink for the Claude Code runtime
├── README.md                    # the kit's front door
├── .env.example                 # the shape of the environment file
└── PLAYBOOK_*.md                # the seven playbooks the kit ships
```

The layout is a **direct image of the package graph**
([03](03-packages-layers-and-udf.md)): `Packages/Core/` holds the shared
packages, `Packages/Feature/` holds the slices, `Apps/` holds the
composition root, `evals/` holds the `.claude` contract tests.

## 2. The package list

There is no `settings.gradle.kts` to edit: a SwiftPM package is a
directory with a manifest, and the **app target** is the only thing that
references them — in `project.yml`.

```yaml
# project.yml — the local packages the app target links
packages:
  Kernel:
    path: Packages/Core/Kernel
  UICore:
    path: Packages/Core/UICore
  NotesContracts:
    path: Packages/Feature/Notes/Contracts
  NotesDomain:
    path: Packages/Feature/Notes/Domain
  NotesData:
    path: Packages/Feature/Notes/Data
  NotesUI:
    path: Packages/Feature/Notes/UI
```

Each package is then declared **twice**, and both declarations are real:

| Declaration | Where | What it fixes |
| --- | --- | --- |
| The package itself | The directory + `Package.swift` | Its products, its targets and its inward dependencies |
| The app target's link | `project.yml` → `packages:` and `targets.<APP>.dependencies:` | Which packages the app links, and the order they are built in |

Adding a package means adding it in both places: the directory and the
manifest, and the reference in `project.yml`. A package that exists but is
not referenced by `project.yml` is never built by the app.

## 3. The package kinds

Four kinds, and the difference is the whole architecture.

| Kind | Which | Declared as | Consequence |
| --- | --- | --- | --- |
| **Pure Swift** | `Kernel`, `UICore`, `NotesContracts`, `NotesDomain` | A library with no UI framework in its manifest | **SwiftUI and UIKit are not on the classpath.** `import SwiftUI` does not compile. |
| **SwiftUI library** | `NotesUI` | A library that imports SwiftUI and Observation | Views, the `@Observable` model, state, events and effects |
| **Adapter** | `NotesData` | A library that depends on its slice's `domain` | Port implementations; the only place a platform SDK is reached |
| **Application** | `<APP>` | `type: application` in `project.yml` | The only target with a bundle identifier, a display name and an `Info.plist` |

The pure packages are what make
[ADR 0002](adr/0002-one-swiftpm-package-per-layer.md) true rather than
aspirational. `scripts/check-architecture.sh` asserts the rules the
compiler cannot see:

| Rule | What it catches |
| --- | --- |
| `Kernel` and `NotesContracts` declare no `.package(path:)` | A pure package that starts depending on another package |
| No `import SwiftUI\|UIKit\|AppKit\|WatchKit` in the pure packages | A framework reaching into a layer that must not see it |
| `NotesUI` ⇏ `NotesData` and `NotesData` ⇏ `NotesUI` | The two forbidden edges SwiftPM cannot see — either direction is buildable |
| `NotesDomain` names neither `NotesData` nor `NotesUI` | The domain reaching an adapter or the presentation |
| Exactly one `type: application` in `project.yml` | A second application target creeping in |
| No `.storyboard`, `.xib`, `.nib`, `.xcdatamodeld`, `.rcproject` | A visual-builder artifact in a code-only tree |
| No tracked `.xcodeproj` | A second source of truth for the project |
| `swiftLanguageModes: [.v6]` in every manifest | A package that drops out of Swift 6 language mode |
| `SWIFT_VERSION: "6.0"` and `SWIFT_STRICT_CONCURRENCY: complete` in `project.yml` | The app target leaving Swift 6 or strict concurrency |

## 4. The version pins

There is no version catalog in this kit, and the reason is not a
preference: **the template has no third-party dependency**. Every package
is local, every framework is a system framework, and the "catalog" is the
set of version pins below.

| What is pinned | Value | Where |
| --- | --- | --- |
| Swift language mode | `6.0` | `SWIFT_VERSION: "6.0"` in `project.yml` |
| Strict concurrency | `complete` | `SWIFT_STRICT_CONCURRENCY: complete` in `project.yml` |
| Swift tools version | `6.0` | `// swift-tools-version: 6.0` in every `Package.swift` |
| Language mode per package | `.v6` | `swiftLanguageModes: [.v6]` in every `Package.swift` |
| Deployment target | iOS `18.0` | `deploymentTarget.iOS` in `project.yml`, `.iOS(.v18)` in every manifest |
| Host platform | macOS `15` | `.macOS(.v15)` in every manifest — it is what makes `swift test` runnable on a Mac |

The pins live in **two** places on purpose, and the gate checks both:
`project.yml` for the app target, and each `Package.swift` for the
packages. A package that compiles under a different language mode than the
app is exactly the failure the strict-concurrency gate exists to prevent.

| Rule | Why |
| --- | --- |
| The version is in the manifest, never in an Xcode setting | The `.xcodeproj` is generated and disposable; the manifest is the source |
| The pins are duplicated, not inherited | A package is a standalone unit: it must build alone, on its own, with `swift test` |
| No remote dependency, so no lockfile | `Package.resolved` is git-ignored — the day a remote package lands, that entry must be dropped and the resolved file committed |

## 5. The manifests

A `Package.swift` is the whole contract of a package. The template's
`NotesUI` manifest, which is the one with the most edges:

```swift
// swift-tools-version: 6.0
let package = Package(
    name: "NotesUI",
    platforms: [.iOS(.v18), .macOS(.v15)],
    products: [
        .library(name: "NotesUI", targets: ["NotesUI"]),
    ],
    dependencies: [
        .package(path: "../Contracts"),
        .package(path: "../Domain"),
        .package(path: "../../../Core/Kernel"),
        .package(path: "../../../Core/UICore"),
    ],
    targets: [
        .target(
            name: "NotesUI",
            dependencies: ["NotesContracts", "NotesDomain", "Kernel", "UICore"]
        ),
        .testTarget(
            name: "NotesUITests",
            dependencies: ["NotesUI", "NotesDomain", "Kernel", "UICore"]
        ),
    ],
    swiftLanguageModes: [.v6]
)
```

Four things to read out of it:

| Element | Why it is there |
| --- | --- |
| The `dependencies` array lists **local paths** | A local package is a path dependency; there is no remote repository to resolve |
| `NotesUI` lists `NotesDomain` explicitly, even though `NotesData` does too | No transitive shortcut: the manifest is the architectural contract, so every edge is written down |
| The product name equals the target name equals the directory name | The graph is readable in `project.yml` without a lookup |
| A `testTarget` ships with the package | The test is part of the package, not an afterthought in the app |

`NotesContracts` and `Kernel` have **no** `dependencies` array at all, and
`check-architecture.sh` fails the build if one of them grows a
`.package(path:)`.

## 6. The app target and XcodeGen

The app target is declared in `project.yml`, and the `.xcodeproj` is
**generated and git-ignored** ([ADR 0001](adr/0001-xcodegen-generated-project.md)).

```yaml
name: <APP>
options:
  bundleIdPrefix: com.<scope>
  deploymentTarget:
    iOS: "18.0"
  createIntermediateGroups: true
settings:
  base:
    SWIFT_VERSION: "6.0"
    SWIFT_STRICT_CONCURRENCY: complete
    ENABLE_USER_SCRIPT_SANDBOXING: YES
    GENERATE_INFOPLIST_FILE: YES
targets:
  <APP>:
    type: application
    platform: iOS
    sources:
      - path: Apps/<APP>/Sources
    dependencies:
      - package: Kernel
      - package: UICore
      - package: NotesContracts
      - package: NotesDomain
      - package: NotesData
      - package: NotesUI
    settings:
      base:
        PRODUCT_BUNDLE_IDENTIFIER: com.<scope>.<app>
        INFOPLIST_KEY_CFBundleDisplayName: <APP>
        INFOPLIST_KEY_UILaunchScreen_Generation: YES
        INFOPLIST_KEY_UIApplicationSceneManifest_Generation: YES
        CURRENT_PROJECT_VERSION: "1"
        MARKETING_VERSION: "0.1.0"
```

The decisions in that manifest, and what each one buys:

| Setting | Effect |
| --- | --- |
| `sources: Apps/<APP>/Sources` | The whole directory is the target's input: a new Swift file is in the app without a project edit |
| `GENERATE_INFOPLIST_FILE: YES` | There is no `Info.plist` file in the tree; the keys are `INFOPLIST_KEY_*` settings |
| `INFOPLIST_KEY_UILaunchScreen_Generation: YES` | The launch screen is generated, so there is no storyboard — the tree stays code-only |
| `ENABLE_USER_SCRIPT_SANDBOXING: YES` | A build phase cannot reach outside the sandbox by accident |
| `bundleIdPrefix` + `PRODUCT_BUNDLE_IDENTIFIER` | `com.<scope>.<app>` — the id the store and the build know the app by |
| `MARKETING_VERSION` / `CURRENT_PROJECT_VERSION` | The version and the build, in the manifest rather than in a shared Xcode setting |

**A build setting is changed in `project.yml`, never in Xcode.** The
generated project is an artifact: `make clean` deletes it, `xcodegen
generate` recreates it, and `check-architecture.sh` fails when an
`.xcodeproj` is tracked in git.

## 7. The commands

| Command | What it does |
| --- | --- |
| `make help` | The target list |
| `make generate` | `xcodegen generate` — writes the `.xcodeproj` from `project.yml` |
| `make build` | `bash scripts/native-gates.sh build` — `xcodegen generate` then `xcodebuild` |
| `make test` | `bash scripts/native-gates.sh test` — `swift test` in each of the six packages |
| `make evals` | `node --test evals/contracts.test.mjs` — the `.claude` contract tests |
| `make lint` | `check-architecture.sh`; after rename, the three `.claude` linters + `instruction-budget.sh` |
| `make clean` | Removes `build`, `DerivedData`, `*.xcodeproj` and every `Packages/**/.build` |
| `make verify` | `bash scripts/verify-template.sh` — stages a renamed copy under `/tmp` and runs every gate on it |
| `make ci` | `lint` + the contract evals + `native-gates.sh` |
| `bash scripts/check-architecture.sh` | The architecture gate, on its own |
| `bash scripts/native-gates.sh [test\|build\|all]` | The Apple-toolchain tier, on its own |
| `node --test evals/contracts.test.mjs` | The contract evals, on their own |
| `swift test` (in a package directory) | The unit tests of one package |
| `xcodegen generate` | The project file, on its own |

The `Makefile` wraps the same gates in shorter names, so a CI job and a
developer run the same commands.

Two facts worth knowing, because they are the usual cause of a confusing
failure:

- **The native tier is macOS-only.** `scripts/native-gates.sh` checks
  `uname -s` and then looks for `xcodegen`, `swift` and `xcodebuild` on
  `PATH`. Anything missing prints `SKIP — …` and exits **0**. A green
  `make ci` on Linux is a green *portable* tier, not a green build; the SKIP
  must be reported, never hidden.
- **The `swift test` tier runs in the package directory.** Each package is
  its own SwiftPM package with its own `Tests/` directory, so
  `swift test` in `Packages/Feature/Notes/Domain` runs that package's
  suite and nothing else.

## 8. The gate

```sh
bash scripts/check-architecture.sh        # the package graph
node --test evals/contracts.test.mjs      # the .claude contract tests
bash scripts/native-gates.sh              # swift test per package + xcodegen + xcodebuild
```

`make ci` runs the lint set, the contract evals and the native tier in one
go. A linter the kit ships is a linter the gate runs — otherwise it is a
linter nobody reads.

The order matters only for the reader: the architecture check is the
cheapest and catches the mistake that would waste the rest of the run.

`bash scripts/verify-template.sh` is the gate above the gate: it copies the
kit to a temporary directory, runs `scripts/rename.sh` on the copy with the
example values `demo` and `demoorg`, and runs the whole gate set on the
renamed result — the `.claude` linters, the instruction budget, the leak
guard, a shell syntax check, the architecture check, the contract evals and
the native tier. That is what proves the substitution contract: that the kit
still passes after it stops being a template. The working tree is never
mutated.

## 9. The new-package adoption path

The template **does not ship** a shared configuration layer: each package
declares its own manifest, which is verbose but obvious. That verbosity is
the seam.

The adoption path for a new package:

1. **Create the directory** under `Packages/Core/` (shared) or
   `Packages/Feature/<slice>/` (owned by one slice).
2. **Write the manifest**: `platforms: [.iOS(.v18), .macOS(.v15)]`,
   `swiftLanguageModes: [.v6]`, one `.library` product, the target, and a
   `testTarget`.
3. **Declare the inward dependencies only.** Pure package → the standard
   library; adapter → its slice's `domain`; UI → its slice's `domain` and
   `contracts`.
4. **Reference it in `project.yml`**: a `packages:` entry with the path, and
   a `- package:` entry in the app target's `dependencies:` if the app uses
   it.
5. **Add it to the two hardcoded lists in the gates**:
   `scripts/check-architecture.sh` (the `PURE_PACKAGES` array if the package
   is pure, and the Swift 6 manifest list) and `scripts/native-gates.sh`
   (the `PACKAGES` array). A package missing from `native-gates.sh` has
   tests that never run.
6. **Add a row to `docs/dependency-register.md`** if the package pulls a
   remote dependency.
7. **Run the gates**: `swift test` in the package, `make ci`.

| What the package boundary buys | What it costs |
| --- | --- |
| A forbidden import fails to compile | A manifest per package, and a path to keep in step |
| The domain is testable off-device with `swift test` | Six packages mean six modules to resolve and compile |
| The graph is a small, reviewable artifact | A new package touches four files, not one |

Do not add a package before the repetition is real. One slice with four
packages plus two shared packages does not need a package factory.

## 10. Starting a project from the template

```sh
cp -a mobile-ios/ my-app/        # copy, then work in the copy
cd my-app/
bash scripts/rename.sh <app> <scope>
xcodegen generate                 # or: make generate
make ci
```

`scripts/rename.sh` substitutes the three placeholder tokens across every
text file, and the renamed copy is where the build happens — the template
root keeps the literal tokens so the kit stays a template.

The script takes the new app slug first and the new organisation scope
second:

```sh
bash scripts/rename.sh <new-app-slug> <new-org-scope>
```

Both values become bundle-identifier segments (`com.<scope>.<app>`) and a
target name, so the script accepts only a lowercase identifier — a letter
first, then letters or digits — and **rejects anything else before it
touches the tree**:

| Token | Replaced by | Where it lands |
| --- | --- | --- |
| `<scope>` | the organisation scope | the bundle-id prefix, `com.<scope>` |
| `<APP>` | the app slug, uppercased | the Xcode target, the display name, the `.xcodeproj` name |
| `<app>` | the app slug, as given | the bundle-id segment and the evals package name |

The validation is `^[a-z][a-z0-9]*$` for both arguments. A hyphenated slug
would leave an unbuildable bundle id behind
(`com.acme.my-app` is not a valid identifier), and a slash would abort the
substitution half way through the copy, leaving a half-renamed tree.

The substitution order is `<scope>` → `<APP>` → `<app>`, and two paths are
**excluded** from it: `scripts/*` and `evals/fixtures/*`. The scripts keep
their literal tokens because they are the tools that perform the
substitution, and the fixtures are the frozen before-and-after samples of it
— they carry deliberate residual tokens so the leak detector keeps working
after the rename.

A second pass renames files whose basename carries a token, so
`Sources/<APP>App.swift` becomes `Sources/MycompanyApp.swift` — the body and
the filename stay in step, and no angle-bracketed file reference reaches the
generated project. A third pass renames the directories, deepest first, so
`Apps/<APP>` becomes `Apps/Mycompany` without breaking its children.

## Do / don't

| Do | Don't |
| --- | --- |
| Change a build setting in `project.yml` | Edit the generated `.xcodeproj` |
| Declare the platform and the language mode in every manifest | Let one package drift to a different mode |
| Add a package to `project.yml` and to the gate lists | Leave a package out of `native-gates.sh` |
| Run `swift test` in the package directory | Assume one command runs every package's suite |
| Report the native tier's SKIP | Present a Linux `make ci` as a successful build |
| Rename on a copy, then work in the copy | Rename the template root in place |

## See also

- [Packages, Layers & UDF](03-packages-layers-and-udf.md) — what each
  package may depend on.
- [Vertical Slicing](04-vertical-slicing.md) — the shape of a slice.
- [Testing & CI](12-testing-and-ci.md) — the gates in detail.
- [ADR 0001 — XcodeGen-generated project](adr/0001-xcodegen-generated-project.md)
  — why the project file is generated.
- [Dependency Register](dependency-register.md) — every dependency and its
  removal path.
