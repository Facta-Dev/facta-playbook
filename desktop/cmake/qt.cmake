# Qt discovery and the shared test helper.
include_guard(GLOBAL)

find_package(Qt6 REQUIRED COMPONENTS Core Gui Widgets Network Sql)

qt_standard_project_setup()

# Qt Test is required only when the suite is built: a missing Qt6::Test must
# fail the configure step, never silently produce a build with zero tests.
if(ENABLE_TESTS)
  find_package(Qt6 REQUIRED COMPONENTS Test)
endif()

# desktop_add_test(<name> <sources>...)
#
# Builds a Qt Test executable, links it against Qt6::Test plus the layer
# libraries and registers it with CTest. QT_QPA_PLATFORM=offscreen keeps the
# suite runnable headless (CI has no display).
function(desktop_add_test name)
  qt_add_executable(${name} ${ARGN})
  target_link_libraries(${name} PRIVATE Qt6::Test ${DESKTOP_TEST_LIBRARIES})
  add_test(NAME ${name} COMMAND ${name})
  set_tests_properties(${name} PROPERTIES ENVIRONMENT "QT_QPA_PLATFORM=offscreen")
endfunction()
