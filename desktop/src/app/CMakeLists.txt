# App shell: the presentation layer of the shell (views and view models).
# main.cpp is compiled by the root executable target.
#
# Source list is owned by the layer agent; keep this ORDER.

set(DESKTOP_PRESENTATION_SOURCES
    include/desktop/app/notes_window.h
    src/notes_window.cpp)

add_library(desktop_presentation STATIC ${DESKTOP_PRESENTATION_SOURCES})
target_include_directories(desktop_presentation PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

# Views talk to view models and to the UI kit; never to the SDK, the storage
# or the network.
target_link_libraries(desktop_presentation
    PUBLIC desktop_application desktop_notes_presentation desktop_ui_kit Qt6::Widgets)

target_compile_definitions(desktop_presentation PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_FOREACH
    QT_DISABLE_DEPRECATED_UP_TO=0x060000)

desktop_assert_no_forbidden_include(desktop_presentation "${CMAKE_CURRENT_SOURCE_DIR}"
    "infrastructure|sdk|QSql")
