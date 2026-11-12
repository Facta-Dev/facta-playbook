# Application version and build metadata.
#
# Override at configure time:
#   cmake -S . -B build -DDESKTOP_VERSION=1.2.3
set(DESKTOP_VERSION "1.0.0" CACHE STRING "Application version (MAJOR.MINOR.PATCH)")

if(NOT DESKTOP_VERSION MATCHES "^[0-9]+\\.[0-9]+\\.[0-9]+$")
  message(FATAL_ERROR "DESKTOP_VERSION must be MAJOR.MINOR.PATCH, got '${DESKTOP_VERSION}'")
endif()

# Available to every target as the string literal "1.0.0".
add_compile_definitions(DESKTOP_VERSION="${DESKTOP_VERSION}")
