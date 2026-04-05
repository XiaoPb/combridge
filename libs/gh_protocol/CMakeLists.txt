cmake_minimum_required(VERSION 3.10)
project(gh_protocol C)

set(CMAKE_C_STANDARD 11)
set(CMAKE_C_STANDARD_REQUIRED ON)

set(SOURCE_FILES
    src/staticmapimp.c
    src/slabmemory.c
    src/gh_rpccore.c
    src/gh_package.c
    src/gh_protocol_api.c
    impl/gh3036/gh_data_package.c
)

include_directories(
    ${CMAKE_SOURCE_DIR}/inc
    ${CMAKE_SOURCE_DIR}/impl/gh3036
)

add_library(gh_protocol STATIC ${SOURCE_FILES})

option(BUILD_EXAMPLE "Build example" ON)

if(BUILD_EXAMPLE)
    add_subdirectory(example)
endif()
