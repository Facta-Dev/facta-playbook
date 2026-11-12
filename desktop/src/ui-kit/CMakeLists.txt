# UI kit: shared widgets, styles, icons, delegates, dialogs.
#
# Source list is owned by the layer agent; keep this ORDER.

set(DESKTOP_UI_KIT_SOURCES
    include/desktop/ui_kit/labeled_line_edit.h
    include/desktop/ui_kit/list_panel.h
    include/desktop/ui_kit/theme.h
    src/labeled_line_edit.cpp
    src/list_panel.cpp
    src/theme.cpp)

add_library(desktop_ui_kit STATIC ${DESKTOP_UI_KIT_SOURCES})
target_include_directories(desktop_ui_kit PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

# The UI kit is presentation-only: widgets, no application layer.
target_link_libraries(desktop_ui_kit PUBLIC Qt6::Widgets)

target_compile_definitions(desktop_ui_kit PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_FOREACH
    QT_DISABLE_DEPRECATED_UP_TO=0x060000)

# The kit carries no business logic.
desktop_assert_no_forbidden_include(desktop_ui_kit "${CMAKE_CURRENT_SOURCE_DIR}"
    "domain|application|features|infrastructure|sdk|contracts")
