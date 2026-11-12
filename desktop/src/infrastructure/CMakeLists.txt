# Infrastructure: outbound adapters (SDK, realtime, storage, telemetry).
#
# Source list is owned by the layer agent; keep this ORDER.

set(DESKTOP_INFRASTRUCTURE_SOURCES
    include/desktop/infrastructure/http/http_transport.h
    src/http/http_transport.cpp
    include/desktop/infrastructure/storage/note_store.h
    src/storage/note_store.cpp
    include/desktop/infrastructure/telemetry/logger.h
    src/telemetry/logger.cpp)

add_library(desktop_infrastructure STATIC ${DESKTOP_INFRASTRUCTURE_SOURCES})
target_include_directories(desktop_infrastructure PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

# Outbound adapters drive the application through its ports and may use the
# platform layer; they never see the views.
#
# desktop_contracts carries the DTOs the storage adapter persists; the SDK
# stays out of this layer.
target_link_libraries(desktop_infrastructure
    PUBLIC desktop_application desktop_platform desktop_contracts Qt6::Network Qt6::Sql)

target_compile_definitions(desktop_infrastructure PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_FOREACH
    QT_DISABLE_DEPRECATED_UP_TO=0x060000)

desktop_assert_no_forbidden_include(desktop_infrastructure "${CMAKE_CURRENT_SOURCE_DIR}"
    "presentation|ui_kit|QWidget|QMainWindow|QLabel|QApplication")

# The runtime check of the layers this target owns.
if(ENABLE_TESTS)
  desktop_add_test(desktop_runtime_tests tests/tst_runtime.cpp)
endif()
