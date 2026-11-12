# Compiler and linker hardening flags (playbook Appendix B).
#
# Deliberately without -Werror: third-party headers (Qt) must not be able to
# break the build, and the warnings stay visible in the CI log.
include(CheckCXXCompilerFlag)

add_compile_options(-Wall -Wextra -Wpedantic)

if(MSVC)
  add_compile_options(/W4 /sdl /GS /guard:cf /permissive- /Zc:__cplusplus)
  add_link_options(/DYNAMICBASE /NXCOMPAT /HIGHENTROPYVA /guard:cf)
else()
  # Static layer libraries may end up inside a shared object.
  set(CMAKE_POSITION_INDEPENDENT_CODE ON)

  check_cxx_compiler_flag(-fstack-protector-strong DESKTOP_HAS_STACK_PROTECTOR)
  if(DESKTOP_HAS_STACK_PROTECTOR)
    add_compile_options(-fstack-protector-strong)
  endif()

  check_cxx_compiler_flag(-fcf-protection DESKTOP_HAS_CF_PROTECTION)
  if(DESKTOP_HAS_CF_PROTECTION)
    add_compile_options(-fcf-protection)
  endif()

  add_link_options(-Wl,-z,relro,-z,now)
endif()
