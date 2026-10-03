# A short output fits in the stream buffer, so the error appears only on flush.
set(input "${CMAKE_CURRENT_BINARY_DIR}/output-failure.hsp")
file(WRITE "${input}" "a=1\n")
foreach(mode IN ITEMS "" "--roundtrip")
    execute_process(COMMAND "${FORMATTER}" ${mode}
        INPUT_FILE "${input}" OUTPUT_FILE "/dev/full"
        RESULT_VARIABLE status ERROR_VARIABLE diagnostic)
    if(NOT "${status}" STREQUAL "2" OR NOT diagnostic MATCHES "output write failed")
        message(FATAL_ERROR "${mode}: expected output failure with exit 2; got ${status}: ${diagnostic}")
    endif()
endforeach()
