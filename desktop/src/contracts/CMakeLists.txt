# Contracts: DTOs, error codes, event envelope, policy schema, route table.
#
# Source list is owned by the layer agent; keep this ORDER.

set(DESKTOP_CONTRACTS_SOURCES
    src/note_dto.cpp
    src/error_envelope.cpp
    src/realtime_envelope.cpp
    src/policy.cpp)

add_library(desktop_contracts STATIC ${DESKTOP_CONTRACTS_SOURCES})
target_include_directories(desktop_contracts PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

# Contracts depend on the shared kernel and on nothing else.
target_link_libraries(desktop_contracts PUBLIC desktop_shared_kernel)

target_compile_definitions(desktop_contracts PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_FOREACH
    QT_DISABLE_DEPRECATED_UP_TO=0x060000)

desktop_assert_no_forbidden_include(desktop_contracts "${CMAKE_CURRENT_SOURCE_DIR}"
    "platform|infrastructure|features")
