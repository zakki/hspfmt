cmake_minimum_required(VERSION 3.16)
project(hspfmt LANGUAGES CXX)

if(MSVC)
    add_compile_options(/W4)
else()
    add_compile_options(-Wall -Wextra -Wpedantic)
endif()

add_library(hspfmt_lib hspfmt.cpp)
target_compile_features(hspfmt_lib PUBLIC cxx_std_17)
target_include_directories(hspfmt_lib PUBLIC "${CMAKE_CURRENT_SOURCE_DIR}")
add_executable(hspfmt main.cpp)
target_link_libraries(hspfmt PRIVATE hspfmt_lib)

include(CTest)
if(BUILD_TESTING)
    add_executable(hspfmt_tests ../test/test_hspfmt.cpp)
    target_link_libraries(hspfmt_tests PRIVATE hspfmt_lib)
    add_test(NAME hspfmt COMMAND hspfmt_tests)
    add_test(NAME hspfmt_cli_errors COMMAND ${CMAKE_COMMAND}
        "-DFORMATTER=$<TARGET_FILE:hspfmt>"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/../test/cli_errors.cmake")
    add_test(NAME hspfmt_cli_spacing COMMAND ${CMAKE_COMMAND}
        "-DFORMATTER=$<TARGET_FILE:hspfmt>"
        -P "${CMAKE_CURRENT_SOURCE_DIR}/../test/cli_spacing.cmake")
    if(UNIX AND EXISTS "/dev/full")
        add_test(NAME hspfmt_output_failure COMMAND ${CMAKE_COMMAND}
            "-DFORMATTER=$<TARGET_FILE:hspfmt>"
            -P "${CMAKE_CURRENT_SOURCE_DIR}/../test/output_failure.cmake")
    endif()
    add_executable(hspfmt_corpus ../test/corpus.cpp)
    target_link_libraries(hspfmt_corpus PRIVATE hspfmt_lib)
endif()
