if(NOT DEFINED PROGRAM OR NOT DEFINED SCENARIO)
    message(FATAL_ERROR "PROGRAM and SCENARIO are required")
endif()

execute_process(
    COMMAND "${PROGRAM}" --headless --scenario "${SCENARIO}"
    RESULT_VARIABLE scenario_result
    OUTPUT_VARIABLE scenario_output
    ERROR_VARIABLE scenario_error
)
if(scenario_result EQUAL 0)
    message(FATAL_ERROR "invalid headless scenario unexpectedly succeeded: ${scenario_output}")
endif()
string(FIND "${scenario_error}" "unknown op" error_offset)
if(error_offset EQUAL -1)
    message(FATAL_ERROR "invalid headless scenario omitted the dispatch error: ${scenario_error}")
endif()
