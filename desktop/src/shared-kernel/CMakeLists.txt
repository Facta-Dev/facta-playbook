# Shared kernel: Result, ids, errors, strong types, time helpers.
#
# Source list is owned by the layer agent; keep this ORDER.

# desktop_assert_no_forbidden_include(<layer> <dir> <forbidden-regex>)
#
# Layer hygiene: a layer that includes a header of a layer it may not depend
# on fails the configure step instead of the review. The link rules in the
# root CMakeLists encode the same rules for symbols.
#
# Defined here because the root CMakeLists adds this directory first.
function(desktop_assert_no_forbidden_include layer dir forbidden)
  file(GLOB_RECURSE layer_sources CONFIGURE_DEPENDS
       "${dir}/src/*.cpp" "${dir}/src/*.h" "${dir}/include/*.h")
  foreach(layer_source IN LISTS layer_sources)
    file(READ "${layer_source}" layer_content)
    if(layer_content MATCHES "#[ \t]*include[ \t]*[<\"][^\">]*(${forbidden})")
      message(FATAL_ERROR
        "${layer} must not include headers matching '${forbidden}': ${layer_source}")
    endif()
  endforeach()
endfunction()

set(DESKTOP_SHARED_KERNEL_SOURCES
    src/error_codes.cpp
    src/timestamp.cpp)

add_library(desktop_shared_kernel STATIC ${DESKTOP_SHARED_KERNEL_SOURCES})
target_include_directories(desktop_shared_kernel PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}/include")

# The kernel is the root of the dependency graph: QtCore value types only.
target_link_libraries(desktop_shared_kernel PUBLIC Qt6::Core)

target_compile_definitions(desktop_shared_kernel PRIVATE
    QT_NO_CAST_FROM_ASCII
    QT_NO_CAST_TO_ASCII
    QT_NO_FOREACH
    QT_DISABLE_DEPRECATED_UP_TO=0x060000)

desktop_assert_no_forbidden_include(desktop_shared_kernel "${CMAKE_CURRENT_SOURCE_DIR}"
    "contracts|platform|infrastructure|features")
