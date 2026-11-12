# Test suites. The tests agent owns the sources and replaces this body.
#
# Every suite is registered with the shared helper, which links Qt6::Test plus
# the layer libraries and runs the test headless:
#
#   desktop_add_test(desktop_domain_tests tst_domain.cpp)
#
# Qt Test is required at configure time whenever ENABLE_TESTS is ON, so a
# missing Qt6::Test fails the configure instead of producing a green build
# with zero tests.

# The fakes shared by the suites: a port is never tested against a real one.
set(DESKTOP_TESTS_FAKES_DIR "${CMAKE_CURRENT_SOURCE_DIR}/_shared/fakes")

# The suites, one executable per layer.
desktop_add_test(desktop_shared_kernel_tests shared-kernel/tst_shared_kernel.cpp)
desktop_add_test(desktop_contracts_tests contracts/tst_contracts.cpp)
desktop_add_test(desktop_notes_slice_tests features/notes/tst_notes_slice.cpp)
desktop_add_test(desktop_notes_view_model_tests
                 features/notes/presentation/tst_notes_view_model.cpp)
desktop_add_test(desktop_notes_window_tests app/tst_notes_window.cpp)
desktop_add_test(desktop_architecture_tests _platform/architecture/tst_architecture.cpp)

# The suites that talk to a fake repository.
foreach(fake_suite desktop_notes_slice_tests desktop_notes_view_model_tests
                   desktop_notes_window_tests)
  target_include_directories(${fake_suite} PRIVATE "${DESKTOP_TESTS_FAKES_DIR}")
endforeach()

# The architecture guard reads the source tree and the QtWidgets headers, so it
# needs the paths of both.
get_target_property(DESKTOP_QT_WIDGETS_INCLUDES Qt6::Widgets INTERFACE_INCLUDE_DIRECTORIES)
set(DESKTOP_QT_WIDGETS_INCLUDE_DIR "")
foreach(include_dir IN LISTS DESKTOP_QT_WIDGETS_INCLUDES)
  if(EXISTS "${include_dir}/QWidget" OR EXISTS "${include_dir}/qwidget.h")
    set(DESKTOP_QT_WIDGETS_INCLUDE_DIR "${include_dir}")
    break()
  endif()
endforeach()
if(NOT DESKTOP_QT_WIDGETS_INCLUDE_DIR)
  message(FATAL_ERROR "the include directory of QtWidgets was not found")
endif()

target_compile_definitions(desktop_architecture_tests PRIVATE
    DESKTOP_SOURCE_ROOT="${CMAKE_SOURCE_DIR}"
    DESKTOP_QT_WIDGETS_INCLUDE_DIR="${DESKTOP_QT_WIDGETS_INCLUDE_DIR}")
