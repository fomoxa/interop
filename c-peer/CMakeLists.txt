cmake_minimum_required(VERSION 3.16)
project(c-peer LANGUAGES C)

include(FetchContent)
FetchContent_Declare(
  fomoxa
  GIT_REPOSITORY https://github.com/fomoxa/c
  GIT_TAG main
)
FetchContent_MakeAvailable(fomoxa)

add_executable(c-peer main.c)
target_include_directories(c-peer PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(c-peer PRIVATE fomoxa::fomoxa)

set_target_properties(c-peer PROPERTIES
    C_STANDARD 99
    C_STANDARD_REQUIRED ON)
