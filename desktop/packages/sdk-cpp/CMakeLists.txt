# C++ SDK: generated client plus the thin hand-written transport layer.
#
# Source list is owned by the layer agent; keep this ORDER.

set(DESKTOP_SDK_SOURCES
    include/desktop/sdk/notes_client.h
    src/notes_client.cpp)

add_library(desktop_sdk STATIC ${DESKTOP_SDK_SOURCES})
target_include_directories(desktop_sdk PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

# The SDK is GUI-free: Core and Network only. The contracts carry the wire
# shapes; the SDK never sees the domain of a slice.
target_link_libraries(desktop_sdk PUBLIC Qt6::Core Qt6::Network desktop_contracts)

target_compile_definitions(desktop_sdk PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_FOREACH
    QT_DISABLE_DEPRECATED_UP_TO=0x060000)

desktop_assert_no_forbidden_include(desktop_sdk "${CMAKE_CURRENT_SOURCE_DIR}"
    "presentation|ui_kit|QWidget|QMainWindow|QApplication|features|domain|application")
