set(test_dir "${CMAKE_CURRENT_BINARY_DIR}/cli-spacing")
file(MAKE_DIRECTORY "${test_dir}")
get_filename_component(example "${CMAKE_CURRENT_LIST_DIR}/../.hspfmt.example" ABSOLUTE)
get_filename_component(presets "${CMAKE_CURRENT_LIST_DIR}/../presets" ABSOLUTE)

function(expect_output source expected)
    file(WRITE "${test_dir}/source.hsp" "${source}")
    execute_process(COMMAND "${FORMATTER}" ${ARGN} source.hsp
        WORKING_DIRECTORY "${test_dir}"
        RESULT_VARIABLE status OUTPUT_VARIABLE output ERROR_VARIABLE diagnostic)
    if(NOT "${status}" STREQUAL "0" OR NOT "${output}" STREQUAL "${expected}" OR NOT "${diagnostic}" STREQUAL "")
        message(FATAL_ERROR "${ARGN}: ${status}: expected '${expected}', got '${output}${diagnostic}'")
    endif()
endfunction()

set(source "\tt1=1:t2=2\n\tobjsize 120,20\t\t; aligned\n*main\n\trepeat\n\tx=1\n\tloop\n")
expect_output("${source}" "\tt1=1 : t2=2\n\tobjsize 120,20 ; aligned\n*main\n\trepeat\n\tx=1\n\tloop\n"
    "--config=${example}")
expect_output("${source}" "\tt1=1:t2=2\n\tobjsize 120,20 ; aligned\n*main\n\trepeat\n\t\tx=1\n\tloop\n"
    "--config=${presets}/compact.hspfmt")
expect_output("${source}" "t1=1 : t2=2\nobjsize 120,20 ; aligned\n*main\n\trepeat\n\t\tx=1\n\tloop\n"
    "--config=${presets}/structured.hspfmt")
# Whitespace differences converge to the same preset output.
set(variant " t1 = 1 : t2 = 2\nobjsize 120 , 20 ; aligned\n*main\nrepeat\n        x = 1\nloop\n")
expect_output("${variant}" "\tt1=1 : t2=2\n\tobjsize 120,20 ; aligned\n*main\n\trepeat\n\tx=1\n\tloop\n"
    "--config=${example}")
expect_output("${variant}" "\tt1=1:t2=2\n\tobjsize 120,20 ; aligned\n*main\n\trepeat\n\t\tx=1\n\tloop\n"
    "--config=${presets}/compact.hspfmt")
expect_output("${variant}" "t1=1 : t2=2\nobjsize 120,20 ; aligned\n*main\n\trepeat\n\t\tx=1\n\tloop\n"
    "--config=${presets}/structured.hspfmt")
expect_output("${source}" "t1 = 1 : t2 = 2\nobjsize 120, 20 ; aligned\n*main\nrepeat\n\tx = 1\nloop\n"
    "--config=${example}" --base-indent=0 --loop-indent=1 --operator-spacing=space --comma-spacing=space --colon-spacing=space --comment-spacing=space)
expect_output("\trepeat\nx=1\n\tloop\n" "repeat\n  x = 1\nloop\n"
    --no-config --indent=preserve --indent=2)
expect_output("a = 1:b=2\n" "a=1 : b=2\n" --no-config --operator-spacing=preserve --compact-operators)
expect_output("a = 1:b=2\n" "a = 1 : b=2\n" --no-config --compact-operators --operator-spacing=preserve)
expect_output("mes 1, 2 : mes 3 ; tail\n" "mes 1,2:mes 3; tail\n"
    --no-config --comma-spacing=compact --colon-spacing=compact --comment-spacing=compact)

foreach(option operator-spacing comma-spacing colon-spacing comment-spacing)
    execute_process(COMMAND "${FORMATTER}" --no-config "--${option}=invalid" source.hsp
        WORKING_DIRECTORY "${test_dir}"
        RESULT_VARIABLE status OUTPUT_VARIABLE output ERROR_VARIABLE diagnostic)
    string(FIND "${diagnostic}" "invalid spacing mode" found)
    if(NOT "${status}" STREQUAL "2" OR NOT "${output}" STREQUAL "" OR found EQUAL -1)
        message(FATAL_ERROR "${option} accepted an invalid spacing mode: ${status}: ${output}${diagnostic}")
    endif()
endforeach()

foreach(option base-indent loop-indent)
    execute_process(COMMAND "${FORMATTER}" --no-config "--${option}=17" source.hsp
        WORKING_DIRECTORY "${test_dir}"
        RESULT_VARIABLE status OUTPUT_VARIABLE output ERROR_VARIABLE diagnostic)
    string(REPLACE "-" " " description "${option}")
    string(FIND "${diagnostic}" "${description} must be between 0 and 16" found)
    if(NOT "${status}" STREQUAL "2" OR NOT "${output}" STREQUAL "" OR found EQUAL -1)
        message(FATAL_ERROR "${option} accepted an invalid depth: ${status}: ${output}${diagnostic}")
    endif()
endforeach()
