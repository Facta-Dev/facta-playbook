cmake_minimum_required(VERSION 3.24)

# Evals harness (EVALS=ON). The project name is the rename-state marker read
# by the post-rename sweep in contracts_test.cpp.
project(<app>-evals LANGUAGES CXX)

# Qt Test is required by the evals binary even when the app test suites are
# off: a missing Qt6::Test must fail the configure step, never silently
# produce a build with zero evals.
find_package(Qt6 REQUIRED COMPONENTS Test)

# Standalone build: `cmake -S evals -B <build>` from the template root. The
# root project cannot be configured as shipped (its executable target still
# carries the literal placeholder name), so the evals tree carries the
# minimum it needs to be configured and run on its own. Inside the root
# project the root CMakeLists.txt already did all of this.
if(PROJECT_IS_TOP_LEVEL)
  set(CMAKE_CXX_STANDARD 20)
  set(CMAKE_CXX_STANDARD_REQUIRED ON)
  set(CMAKE_AUTOMOC ON)
  qt_standard_project_setup()
  if(CMAKE_BUILD_TYPE STREQUAL "Debug")
    set(_evals_tests_default ON)
  else()
    set(_evals_tests_default OFF)
  endif()
  option(ENABLE_TESTS "Build the Qt Test suites" ${_evals_tests_default})
  if(ENABLE_TESTS)
    enable_testing()
  endif()
endif()

# The evals are a contract harness, not an app layer: they read the template
# tree from disk and depend on Qt6::Test alone (no presentation, no domain).
qt_add_executable(desktop_evals_contracts contracts_test.cpp)
target_link_libraries(desktop_evals_contracts PRIVATE Qt6::Test)

# The tree the evals validate is the tree they were built from (the staged
# copy under scripts/verify-template.sh, or the kit root itself).
target_compile_definitions(desktop_evals_contracts PRIVATE
  DESKTOP_EVALS_DIR="${CMAKE_CURRENT_SOURCE_DIR}")

if(ENABLE_TESTS)
  add_test(NAME desktop_evals_contracts COMMAND desktop_evals_contracts)
  set_tests_properties(desktop_evals_contracts PROPERTIES
    ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
endif()
