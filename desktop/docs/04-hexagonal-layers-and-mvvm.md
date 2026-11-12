# 04 — Hexagonal Layers & MVVM

## The layers

The template has one static library per layer, so that the dependency rule is
a link-time fact and not a comment. The targets and what they may depend on:

| Target | Layer | May link | Must not know |
| --- | --- | --- | --- |
| `desktop_shared_kernel` | Kernel | Qt Core only | Everything |
| `desktop_contracts` | Contracts | Kernel | Qt Widgets, SQL, HTTP |
| `desktop_domain` | Domain | Kernel, Contracts | Widgets, SQL, HTTP, the OS |
| `desktop_application` | Application | Domain | Adapters, Widgets, SQL |
| `desktop_platform` | Platform | Kernel | The domain, the application layer |
| `desktop_infrastructure` | Infrastructure | Application, Platform, Contracts | The view |
| `desktop_ui_kit` | UI kit | — | The domain, the application layer |
| `desktop_presentation` | Presentation | Application, Slice view models, UI kit | The domain's storage, SQL |
| `desktop_sdk` | SDK | Contracts, Qt Core, Qt Network | Widgets, SQL |
| `desktop_notes_adapters` | Slice adapters | Application, Infrastructure, SDK | The view |
| `desktop_notes_presentation` | Slice view model | Application | The view, SQL, HTTP |
| `<app>-desktop` | Executable | Everything | — it *is* the composition root |

The rules are in the root [`CMakeLists.txt`](../CMakeLists.txt); the full
table is in [ADR 0002](adr/0002-one-static-library-per-layer.md).

## The port

The application layer declares the interface it needs; the adapter implements
it. The interface is written in terms of **domain** types, never in terms of
rows or JSON:

```cpp
class NoteRepository {
public:
    virtual ~NoteRepository() = default;
    [[nodiscard]] virtual kernel::Result<QVector<domain::Note>> list() const = 0;
    [[nodiscard]] virtual kernel::Result<domain::Note> add(const domain::Note& note) = 0;
};
```

Three properties make it a port and not a wrapper: **it is owned by the
consumer** (the interface lives next to the use cases that need it), **it
speaks domain types** (`domain::Note`, not a row, not a DTO), and **it
carries no framework type** (no `QSqlQuery`, no `QNetworkReply`, no
`QFuture` — for now; see [07](07-threading-and-the-gui-thread.md)).

## Errors travel as values

`desktop::kernel::Result<T>` is a `std::variant` of a value or an error.
No exception crosses a port, and no error code is invented by the caller:
the use case forwards the code it received.

`ErrorCode` is a closed enum in the kernel, and each code has a **wire
name** in `src/shared-kernel/src/error_codes.cpp`, so the same failure reads
the same on the wire and in the log. The wire envelope is
`desktop::contracts::ErrorEnvelope`: `{code, detail, requestId,
requiredPermission}`.

## MVVM in Qt

The view model is a `QObject` that holds the state, exposes it as
properties, and takes the commands. The view is a widget that renders it.

```cpp
class NotesViewModel : public QObject {
    Q_OBJECT
    Q_PROPERTY(State state READ state NOTIFY stateChanged)
    Q_PROPERTY(QString statusMessage READ statusMessage NOTIFY stateChanged)
    Q_PROPERTY(int count READ count NOTIFY countChanged)
    Q_INVOKABLE void refresh();
    Q_INVOKABLE void addNote(const QString& title, const QString& body);
};
```

| MVVM | Qt | In the template |
| --- | --- | --- |
| Model (domain) | Plain C++ class | `desktop::notes::domain::Note` |
| View | `QWidget` | `desktop::app::NotesWindow` |
| ViewModel | `QObject` + `Q_PROPERTY` | `desktop::notes::presentation::NotesViewModel` |
| Item model | `QAbstractItemModel` | `desktop::notes::presentation::NotesListModel` |
| Binding | `NOTIFY` signal | `stateChanged`, `countChanged` |
| Command | `Q_INVOKABLE` slot | `refresh()`, `addNote()` |

**The view model never knows a widget.** It never includes `QWidget`, never
holds a pointer to one, and never calls into one. It emits signals; the view
connects to them. That is what makes it testable without a display server.

**The state is one enum, not a set of booleans.** `State` has eight values
(`Loading`, `Ready`, `Empty`, `Forbidden`, `NotFound`, `Offline`, `Error`,
`PolicyBlocked`) and the view renders exactly one of them. Booleans admit
contradictory states; an enum does not.

## The three guards

| # | Guard | Fires at | Mechanism |
| --- | --- | --- | --- |
| 1 | `desktop_assert_no_forbidden_include` | CMake configure | A regex over the layer's sources; a forbidden `#include` is a `FATAL_ERROR` naming the file |
| 2 | `target_link_libraries` | Link time | An undefined reference to a symbol the layer may not use |
| 3 | `tests/_platform/architecture/tst_architecture.cpp` | Test time | A source scan of the tree, three rules |

Guard 1 is a configure-time net for the include-level mistake:

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

Guard 2 is the real one: it is impossible to call a widget from the domain if
the domain is not linked against Widgets. A `QString` function in the ui-kit
called from the domain produces an undefined reference at link, not a code
review comment.

Guard 3 keeps the tree honest over time — it reads the sources and asserts
the three rules that are not expressible as link rules (a domain file that
mentions `QWidget`, an OS header outside the platform layer, an application
file that names the infrastructure).

## Do / don't

| Do | Don't |
| --- | --- |
| Declare the port next to the use case | Put the interface in the adapter |
| Return the domain type from the port | Return a row or a DTO |
| Keep the view model free of `QWidget` | Give the view model a pointer to the window |
| Model the state as an enum | Model it as five booleans |

## See also

- [ADR 0002 — One static library per layer](adr/0002-one-static-library-per-layer.md)
- [Vertical Slicing](05-vertical-slicing.md) — where the port lives in a slice.
- [Threading & the GUI Thread](07-threading-and-the-gui-thread.md) — the
  port's asynchronous shape.
