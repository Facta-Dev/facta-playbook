# Platform: the ONLY place with OS-specific code (#ifdef Q_OS_* lives here).
#
# Source list is owned by the layer agent; keep this ORDER.

set(DESKTOP_PLATFORM_SOURCES
    src/common/app_paths.cpp
    src/common/single_instance.cpp
    src/common/keystore_unavailable.h
    src/linux/keystore_linux.cpp
    src/windows/keystore_windows.cpp
    src/macos/keystore_macos.cpp)

add_library(desktop_platform STATIC ${DESKTOP_PLATFORM_SOURCES})
target_include_directories(desktop_platform PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

# The platform layer sits above the shared kernel, never above domain or
# application. No OS package is looked up at configure time yet.
#
# Qt6::Network carries QLocalServer/QLocalSocket, the single-instance IPC of
# playbook 10; no X11 or D-Bus header is involved.
target_link_libraries(desktop_platform
    PUBLIC desktop_shared_kernel Qt6::Core Qt6::Gui Qt6::Network)

target_compile_definitions(desktop_platform PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_FOREACH
    QT_DISABLE_DEPRECATED_UP_TO=0x060000)

desktop_assert_no_forbidden_include(desktop_platform "${CMAKE_CURRENT_SOURCE_DIR}"
    "contracts|domain|application|features|ui_kit")
