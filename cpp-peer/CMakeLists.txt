cmake_minimum_required(VERSION 3.16)
project(cpp-peer LANGUAGES CXX)

include(FetchContent)
FetchContent_Declare(
  fomoxa
  GIT_REPOSITORY https://github.com/fomoxa/c
  GIT_TAG main
)
FetchContent_MakeAvailable(fomoxa)

add_executable(cpp-peer main.cpp)
target_include_directories(cpp-peer PRIVATE ${CMAKE_CURRENT_SOURCE_DIR})
target_link_libraries(cpp-peer PRIVATE fomoxa::fomoxa)

set_target_properties(cpp-peer PROPERTIES
    CXX_STANDARD 17
    CXX_STANDARD_REQUIRED ON)
