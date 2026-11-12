# Notes vertical slice: domain, application, adapters and presentation.
#
# Source list is owned by the layer agent; keep this ORDER.

set(DESKTOP_NOTES_DOMAIN_SOURCES
    domain/include/desktop/notes/domain/note.h
    domain/src/note.cpp)

add_library(desktop_domain STATIC ${DESKTOP_NOTES_DOMAIN_SOURCES})
target_include_directories(desktop_domain PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/domain/include")

# The domain owns the invariants of a note and knows nothing else.
target_link_libraries(desktop_domain PUBLIC desktop_shared_kernel desktop_contracts)

set(DESKTOP_NOTES_APPLICATION_SOURCES
    application/include/desktop/notes/application/add_note.h
    application/include/desktop/notes/application/list_notes.h
    application/include/desktop/notes/application/note_repository.h
    application/src/add_note.cpp
    application/src/list_notes.cpp)

add_library(desktop_application STATIC ${DESKTOP_NOTES_APPLICATION_SOURCES})
target_include_directories(desktop_application
    PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/application/include")

# The use cases depend on the domain and on the port, never on an adapter.
target_link_libraries(desktop_application PUBLIC desktop_domain)

set(DESKTOP_NOTES_ADAPTERS_SOURCES
    adapters/include/desktop/notes/adapters/sdk_note_transport.h
    adapters/include/desktop/notes/adapters/sqlite_note_repository.h
    adapters/src/sdk_note_transport.cpp
    adapters/src/sqlite_note_repository.cpp)

add_library(desktop_notes_adapters STATIC ${DESKTOP_NOTES_ADAPTERS_SOURCES})
target_include_directories(desktop_notes_adapters
    PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/adapters/include")

# The adapters are the only place where the slice meets the storage and the
# SDK; the SDK never reaches the domain or the application.
target_link_libraries(desktop_notes_adapters
    PUBLIC desktop_application desktop_infrastructure desktop_sdk)

set(DESKTOP_NOTES_PRESENTATION_SOURCES
    presentation/include/desktop/notes/presentation/notes_list_model.h
    presentation/include/desktop/notes/presentation/notes_view_model.h
    presentation/src/notes_list_model.cpp
    presentation/src/notes_view_model.cpp)

add_library(desktop_notes_presentation STATIC ${DESKTOP_NOTES_PRESENTATION_SOURCES})
target_include_directories(desktop_notes_presentation
    PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/presentation/include")

# The view model is a QObject and an item model: QtCore only, never a widget.
target_link_libraries(desktop_notes_presentation PUBLIC desktop_application Qt6::Core)

foreach(notes_target desktop_domain desktop_application desktop_notes_adapters
                      desktop_notes_presentation)
  target_compile_definitions(${notes_target} PRIVATE
      QT_NO_CAST_FROM_ASCII
      QT_NO_CAST_TO_ASCII
      QT_NO_FOREACH
      QT_DISABLE_DEPRECATED_UP_TO=0x060000)
endforeach()

desktop_assert_no_forbidden_include(desktop_domain "${CMAKE_CURRENT_SOURCE_DIR}/domain"
    "presentation|ui_kit|QWidget|QMainWindow|QLabel|QApplication|adapters|infrastructure|platform|sdk")
desktop_assert_no_forbidden_include(desktop_application "${CMAKE_CURRENT_SOURCE_DIR}/application"
    "presentation|ui_kit|QWidget|QMainWindow|QLabel|QApplication|adapters|infrastructure|platform|sdk")
desktop_assert_no_forbidden_include(desktop_notes_adapters "${CMAKE_CURRENT_SOURCE_DIR}/adapters"
    "presentation|ui_kit|QWidget|QMainWindow|QLabel|QApplication")
desktop_assert_no_forbidden_include(desktop_notes_presentation "${CMAKE_CURRENT_SOURCE_DIR}/presentation"
    "infrastructure|sdk|ui_kit|QWidget|QMainWindow|QLabel|QApplication")

# The runtime check of the slice.
if(ENABLE_TESTS)
  desktop_add_test(desktop_notes_tests tests/tst_notes.cpp)
endif()
