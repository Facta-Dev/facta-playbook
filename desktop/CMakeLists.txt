cmake_minimum_required(VERSION 3.24)

project(<app>-desktop LANGUAGES CXX VERSION 1.0.0)

# C++20 is set explicitly and before qt_standard_project_setup(), which would
# otherwise downgrade the standard to the Qt default.
set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)
set(CMAKE_AUTOMOC ON)
set(CMAKE_EXPORT_COMPILE_COMMANDS ON)

# Tests are on by default in Debug builds; `make ci` turns them on everywhere.
if(CMAKE_BUILD_TYPE STREQUAL "Debug")
  set(_desktop_tests_default ON)
else()
  set(_desktop_tests_default OFF)
endif()
option(ENABLE_TESTS "Build the Qt Test suites" ${_desktop_tests_default})
option(EVALS "Build the evals harness" OFF)

# Testing is enabled before the layer directories are added: a layer that
# registers its own test with add_test() is dropped otherwise, because no
# CTestTestfile.cmake is written for a directory added before enable_testing().
if(ENABLE_TESTS)
  enable_testing()
endif()

include(cmake/versioning.cmake)
include(cmake/hardening.cmake)
include(cmake/qt.cmake)

# Layer libraries are linked with these rules; a violation fails at link time.
#   domain        -> shared_kernel, contracts
#   application   -> domain
#   platform      -> shared_kernel
#   infrastructure-> application, platform
#   slice adapters-> application, infrastructure, sdk
#   slice viewmodels -> application
#   sdk           -> core, network, contracts
#   presentation  -> application, ui_kit
#   app executable-> presentation, infrastructure, platform, slice adapters
set(DESKTOP_TEST_LIBRARIES
    desktop_shared_kernel
    desktop_contracts
    desktop_platform
    desktop_infrastructure
    desktop_domain
    desktop_application
    desktop_notes_adapters
    desktop_notes_presentation
    desktop_sdk
    desktop_presentation
    desktop_ui_kit)

# Placeholder factory for the layer libraries: a static library needs at least
# one source, so a stub is written into the build tree (never committed) and
# replaced by the owning layer agent.
function(desktop_add_placeholder_library name dir)
  set(stub "${CMAKE_CURRENT_BINARY_DIR}/placeholder.cpp")
  file(WRITE "${stub}" "// Generated stub: the layer agent replaces the source list.\n")
  add_library(${name} STATIC "${stub}")
  target_include_directories(${name} INTERFACE "${dir}/include")
endfunction()

add_subdirectory(src/shared-kernel)
add_subdirectory(src/contracts)
add_subdirectory(src/ui-kit)
add_subdirectory(src/platform)
add_subdirectory(src/features/notes)
add_subdirectory(src/infrastructure)
add_subdirectory(packages/sdk-cpp)
add_subdirectory(src/app)

qt_add_executable(<app>-desktop src/app/main.cpp)
target_link_libraries(<app>-desktop
    PRIVATE desktop_presentation desktop_infrastructure desktop_platform
            desktop_notes_adapters)
if(WIN32)
  set_target_properties(<app>-desktop PROPERTIES WIN32_EXECUTABLE TRUE)
elseif(APPLE)
  set_target_properties(<app>-desktop PROPERTIES MACOSX_BUNDLE TRUE)
endif()

if(ENABLE_TESTS)
  add_subdirectory(tests)
endif()

if(EVALS)
  add_subdirectory(evals)
endif()
