set(test_dir "${CMAKE_CURRENT_BINARY_DIR}/cli-errors/explicit")
file(MAKE_DIRECTORY "${test_dir}/config-dir")
set(input "${test_dir}/source.hsp")
set(source "repeat\nx=1\nloop\n")
file(WRITE "${input}" "${source}")
file(WRITE "${test_dir}/.hspfmt" "indent=2\n")
file(WRITE "${test_dir}/valid.hsp" "x = 1\n")
file(WRITE "${test_dir}/invalid.hsp" "repeat\n")
file(WRITE "${test_dir}/invalid-string.hsp" "mes \"unterminated")

function(expect_error fragment)
    execute_process(COMMAND "${FORMATTER}" ${ARGN}
        WORKING_DIRECTORY "${test_dir}" INPUT_FILE "${input}"
        RESULT_VARIABLE status OUTPUT_VARIABLE output ERROR_VARIABLE diagnostic)
    string(FIND "${diagnostic}" "${fragment}" found)
    if(NOT "${status}" STREQUAL "2" OR NOT "${output}" STREQUAL "" OR found EQUAL -1)
        message(FATAL_ERROR "${ARGN}: expected exit 2, no stdout, and '${fragment}'; got ${status}: ${output}${diagnostic}")
    endif()
endfunction()

# A directory can open successfully but fail on read, depending on the platform.
expect_error("config-dir" "--config=config-dir" --write source.hsp)
file(READ "${input}" after)
if(NOT "${after}" STREQUAL "${source}")
    message(FATAL_ERROR "config read failure changed the input file")
endif()

# An empty explicit path must not fall back to the valid automatic config.
expect_error("--config requires a non-empty file path" "--config=" --write source.hsp)
expect_error("--config requires a non-empty file path" --no-config "--config=" source.hsp)
file(READ "${input}" after)
if(NOT "${after}" STREQUAL "${source}")
    message(FATAL_ERROR "empty config path changed the input file")
endif()

expect_error("hspfmt: invalid.hsp: unterminated block" --check valid.hsp invalid.hsp)
expect_error("hspfmt: invalid-string.hsp: unterminated string" --roundtrip invalid-string.hsp)
expect_error("hspfmt: missing.hsp: cannot open input file" --check valid.hsp missing.hsp)
set(input "${test_dir}/invalid.hsp")
expect_error("hspfmt: virtual/path.hsp: unterminated block" --stdin-filepath=virtual/path.hsp -)
expect_error("hspfmt: <stdin>: unterminated block" -)

# Full-width space modes work in configs, with explicit CLI overrides.
set(input "${test_dir}/full-space.hsp")
set(full_space_source "mes　1\n")
file(WRITE "${input}" "${full_space_source}")
file(WRITE "${test_dir}/spaces.hspfmt" "full-width-spaces=normalize\n")
execute_process(COMMAND "${FORMATTER}" --config=spaces.hspfmt full-space.hsp
    WORKING_DIRECTORY "${test_dir}"
    RESULT_VARIABLE status OUTPUT_VARIABLE output ERROR_VARIABLE diagnostic)
if(NOT "${status}" STREQUAL "0" OR NOT "${output}" STREQUAL "mes 1\n" OR NOT "${diagnostic}" STREQUAL "")
    message(FATAL_ERROR "full-width space config failed: ${status}: ${output}${diagnostic}")
endif()
foreach(mode preserve roundtrip)
    if(mode STREQUAL "preserve")
        set(option --full-width-spaces=preserve)
    else()
        set(option --roundtrip)
    endif()
    execute_process(COMMAND "${FORMATTER}" --config=spaces.hspfmt "${option}" full-space.hsp
        WORKING_DIRECTORY "${test_dir}"
        RESULT_VARIABLE status OUTPUT_VARIABLE output ERROR_VARIABLE diagnostic)
    if(NOT "${status}" STREQUAL "0" OR NOT "${output}" STREQUAL "${full_space_source}" OR NOT "${diagnostic}" STREQUAL "")
        message(FATAL_ERROR "${mode} changed full-width spaces: ${status}: ${output}${diagnostic}")
    endif()
endforeach()
expect_error("invalid full-width space mode" --no-config --full-width-spaces=invalid full-space.hsp)

# The automatically loaded .hspfmt must also reject read failures before writing.
set(test_dir "${CMAKE_CURRENT_BINARY_DIR}/cli-errors/automatic")
file(MAKE_DIRECTORY "${test_dir}/.hspfmt")
set(input "${test_dir}/source.hsp")
file(WRITE "${input}" "${source}")
expect_error(".hspfmt" --write source.hsp)
file(READ "${input}" after)
if(NOT "${after}" STREQUAL "${source}")
    message(FATAL_ERROR "automatic config read failure changed the input file")
endif()
