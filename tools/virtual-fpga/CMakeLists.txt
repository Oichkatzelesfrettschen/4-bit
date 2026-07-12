cmake_minimum_required(VERSION 3.20)

project(mcs4_virtual_fpga LANGUAGES CXX)

include(CTest)

set(CMAKE_CXX_STANDARD 20)
set(CMAKE_CXX_STANDARD_REQUIRED ON)
set(CMAKE_CXX_EXTENSIONS OFF)

get_filename_component(REPOSITORY_ROOT "${CMAKE_CURRENT_LIST_DIR}/../.." ABSOLUTE)

find_package(Qt6 6.2 REQUIRED COMPONENTS Core Widgets)
find_package(Python3 REQUIRED COMPONENTS Interpreter)
find_package(Threads REQUIRED)
find_program(CARGO_EXECUTABLE NAMES cargo REQUIRED)
find_program(VERILATOR_EXECUTABLE NAMES verilator REQUIRED)
find_program(MAKE_EXECUTABLE NAMES gmake make REQUIRED)

execute_process(
    COMMAND "${VERILATOR_EXECUTABLE}" --getenv VERILATOR_ROOT
    RESULT_VARIABLE VERILATOR_ROOT_RESULT
    OUTPUT_VARIABLE VERILATOR_ROOT
    OUTPUT_STRIP_TRAILING_WHITESPACE
)
if(NOT VERILATOR_ROOT_RESULT EQUAL 0 OR NOT EXISTS "${VERILATOR_ROOT}/include/verilated.h")
    message(FATAL_ERROR "Unable to locate the Verilator runtime include directory")
endif()

set(GENERATED_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/generated")
set(FPGA_VERILOG "${GENERATED_DIRECTORY}/i4003_fpga.v")
set(FPGA_MANIFEST "${GENERATED_DIRECTORY}/i4003_fpga.v.manifest.json")
file(GLOB_RECURSE FPGA_EXPORT_SOURCES CONFIGURE_DEPENDS
    "${REPOSITORY_ROOT}/mcs4-emu/crates/mcs4-fpga/src/*.rs"
)

add_custom_command(
    OUTPUT "${FPGA_VERILOG}" "${FPGA_MANIFEST}"
    COMMAND "${CARGO_EXECUTABLE}" run --locked -p mcs4-fpga --bin mcs4-fpga-export --
        --chip 4003 --flavor fpga --output "${FPGA_VERILOG}" --manifest "${FPGA_MANIFEST}"
    DEPENDS
        "${REPOSITORY_ROOT}/Cargo.lock"
        "${REPOSITORY_ROOT}/Cargo.toml"
        "${REPOSITORY_ROOT}/mcs4-emu/crates/mcs4-fpga/Cargo.toml"
        ${FPGA_EXPORT_SOURCES}
    WORKING_DIRECTORY "${REPOSITORY_ROOT}"
    VERBATIM
)

set(VERILATED_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/verilated")
set(VERILATED_ARCHIVE "${VERILATED_DIRECTORY}/Vi4003_fpga__ALL.a")

add_custom_command(
    OUTPUT "${VERILATED_ARCHIVE}"
    COMMAND "${CMAKE_COMMAND}" -E make_directory "${VERILATED_DIRECTORY}"
    COMMAND "${VERILATOR_EXECUTABLE}" --cc --trace --Wall
        --Mdir "${VERILATED_DIRECTORY}"
        --top-module i4003_fpga
        "${FPGA_VERILOG}"
    COMMAND "${MAKE_EXECUTABLE}" -C "${VERILATED_DIRECTORY}" -f Vi4003_fpga.mk Vi4003_fpga__ALL.a
    DEPENDS "${FPGA_VERILOG}"
    BYPRODUCTS
        "${VERILATED_DIRECTORY}/Vi4003_fpga.h"
        "${VERILATED_DIRECTORY}/Vi4003_fpga.mk"
    VERBATIM
)

add_custom_target(verilate_i4003_fpga DEPENDS "${VERILATED_ARCHIVE}")

set(GOWIN_DIRECTORY "${REPOSITORY_ROOT}/mcs4-emu/crates/mcs4-fpga/gowin")
set(SYSTEM_FPGA_VERILOGS)
foreach(CHIP IN ITEMS 4004 4001 4002)
    set(SYSTEM_FPGA_VERILOG "${GENERATED_DIRECTORY}/i${CHIP}_fpga.v")
    set(SYSTEM_FPGA_MANIFEST "${GENERATED_DIRECTORY}/i${CHIP}_fpga.v.manifest.json")
    add_custom_command(
        OUTPUT "${SYSTEM_FPGA_VERILOG}" "${SYSTEM_FPGA_MANIFEST}"
        COMMAND "${CARGO_EXECUTABLE}" run --locked -p mcs4-fpga --bin mcs4-fpga-export --
            --chip "${CHIP}" --flavor fpga --output "${SYSTEM_FPGA_VERILOG}" --manifest "${SYSTEM_FPGA_MANIFEST}"
        DEPENDS
            "${REPOSITORY_ROOT}/Cargo.lock"
            "${REPOSITORY_ROOT}/Cargo.toml"
            "${REPOSITORY_ROOT}/mcs4-emu/crates/mcs4-fpga/Cargo.toml"
            ${FPGA_EXPORT_SOURCES}
        WORKING_DIRECTORY "${REPOSITORY_ROOT}"
        VERBATIM
    )
    list(APPEND SYSTEM_FPGA_VERILOGS "${SYSTEM_FPGA_VERILOG}")
endforeach()

set(SYSTEM_VERILATED_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/verilated-system")
set(SYSTEM_VERILATED_ARCHIVE "${SYSTEM_VERILATED_DIRECTORY}/Vmcs4_system_sim_top__ALL.a")
set(SYSTEM_VERILOG_SOURCES
    ${SYSTEM_FPGA_VERILOGS}
    "${GOWIN_DIRECTORY}/clock_gen.v"
    "${GOWIN_DIRECTORY}/rom_bsram.v"
    "${GOWIN_DIRECTORY}/ram_bsram.v"
    "${GOWIN_DIRECTORY}/uart_hw.v"
    "${GOWIN_DIRECTORY}/uart_bridge.v"
    "${GOWIN_DIRECTORY}/mcs4_system_core.v"
    "${GOWIN_DIRECTORY}/mcs4_system_sim_top.v"
)

add_custom_command(
    OUTPUT "${SYSTEM_VERILATED_ARCHIVE}"
    COMMAND "${CMAKE_COMMAND}" -E make_directory "${SYSTEM_VERILATED_DIRECTORY}"
    COMMAND "${VERILATOR_EXECUTABLE}" --cc --trace --Wall -DMCS4_VERILATOR_RUNTIME
        --Mdir "${SYSTEM_VERILATED_DIRECTORY}"
        --top-module mcs4_system_sim_top
        ${SYSTEM_VERILOG_SOURCES}
    COMMAND "${MAKE_EXECUTABLE}" -C "${SYSTEM_VERILATED_DIRECTORY}" -f Vmcs4_system_sim_top.mk Vmcs4_system_sim_top__ALL.a
    DEPENDS ${SYSTEM_VERILOG_SOURCES}
    BYPRODUCTS
        "${SYSTEM_VERILATED_DIRECTORY}/Vmcs4_system_sim_top.h"
        "${SYSTEM_VERILATED_DIRECTORY}/Vmcs4_system_sim_top.mk"
    VERBATIM
)

add_custom_target(verilate_mcs4_system DEPENDS "${SYSTEM_VERILATED_ARCHIVE}")

add_library(verilator_runtime STATIC
    "${VERILATOR_ROOT}/include/verilated.cpp"
    "${VERILATOR_ROOT}/include/verilated_threads.cpp"
    "${VERILATOR_ROOT}/include/verilated_vcd_c.cpp"
)
target_include_directories(verilator_runtime SYSTEM PRIVATE
    "${VERILATOR_ROOT}/include"
    "${VERILATOR_ROOT}/include/vltstd"
)
target_compile_definitions(verilator_runtime PRIVATE
    VERILATOR=1
    VM_COVERAGE=0
    VM_SC=0
    VM_TIMING=0
    VM_TRACE=1
    VM_TRACE_FST=0
    VM_TRACE_VCD=1
    VM_TRACE_SAIF=0
    VM_VPI=0
)

add_executable(mcs4-virtual-fpga main.cpp)
add_dependencies(mcs4-virtual-fpga verilate_i4003_fpga)
target_include_directories(mcs4-virtual-fpga PRIVATE
    "${VERILATED_DIRECTORY}"
)
target_include_directories(mcs4-virtual-fpga SYSTEM PRIVATE
    "${VERILATOR_ROOT}/include"
    "${VERILATOR_ROOT}/include/vltstd"
)
target_compile_definitions(mcs4-virtual-fpga PRIVATE
    VERILATOR=1
    VM_COVERAGE=0
    VM_SC=0
    VM_TIMING=0
    VM_TRACE=1
    VM_TRACE_FST=0
    VM_TRACE_VCD=1
    VM_TRACE_SAIF=0
    VM_VPI=0
)
target_compile_options(mcs4-virtual-fpga PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(mcs4-virtual-fpga PRIVATE
    Qt6::Core
    Qt6::Widgets
    Threads::Threads
    "${VERILATED_ARCHIVE}"
    verilator_runtime
)

add_executable(mcs4-virtual-system system_main.cpp)
add_dependencies(mcs4-virtual-system verilate_mcs4_system)
target_include_directories(mcs4-virtual-system PRIVATE
    "${SYSTEM_VERILATED_DIRECTORY}"
)
target_include_directories(mcs4-virtual-system SYSTEM PRIVATE
    "${VERILATOR_ROOT}/include"
    "${VERILATOR_ROOT}/include/vltstd"
)
target_compile_definitions(mcs4-virtual-system PRIVATE
    VERILATOR=1
    VM_COVERAGE=0
    VM_SC=0
    VM_TIMING=0
    VM_TRACE=1
    VM_TRACE_FST=0
    VM_TRACE_VCD=1
    VM_TRACE_SAIF=0
    VM_VPI=0
)
target_compile_options(mcs4-virtual-system PRIVATE -Wall -Wextra -Wpedantic -Werror)
target_link_libraries(mcs4-virtual-system PRIVATE
    Qt6::Core
    Threads::Threads
    "${SYSTEM_VERILATED_ARCHIVE}"
    verilator_runtime
)

configure_file(
    "${GOWIN_DIRECTORY}/monitor_rom.hex"
    "${CMAKE_CURRENT_BINARY_DIR}/monitor_rom.hex"
    COPYONLY
)

if(BUILD_TESTING)
    set(VIRTUAL_FPGA_TEST_DIRECTORY "${CMAKE_CURRENT_BINARY_DIR}/test-artifacts")
    file(MAKE_DIRECTORY "${VIRTUAL_FPGA_TEST_DIRECTORY}")
    configure_file(
        "${GOWIN_DIRECTORY}/monitor_rom.hex"
        "${VIRTUAL_FPGA_TEST_DIRECTORY}/monitor_rom.hex"
        COPYONLY
    )
    add_test(
        NAME virtual-fpga-i4003-shift-gate
        COMMAND "${CMAKE_COMMAND}"
            "-DPROGRAM=$<TARGET_FILE:mcs4-virtual-fpga>"
            "-DSCENARIO=${CMAKE_CURRENT_LIST_DIR}/scenarios/i4003-shift-gate.json"
            "-DSUMMARY=${VIRTUAL_FPGA_TEST_DIRECTORY}/i4003-summary.json"
            "-DVCD=${VIRTUAL_FPGA_TEST_DIRECTORY}/i4003-shift-gate.vcd"
            "-DTRACE_FRAMES=${VIRTUAL_FPGA_TEST_DIRECTORY}/i4003-shift-gate.trace.jsonl"
            -P "${CMAKE_CURRENT_LIST_DIR}/verify_i4003_scenario.cmake"
    )
    add_test(
        NAME virtual-fpga-i4003-invalid-action
        COMMAND "${CMAKE_COMMAND}"
            "-DPROGRAM=$<TARGET_FILE:mcs4-virtual-fpga>"
            "-DSCENARIO=${CMAKE_CURRENT_LIST_DIR}/scenarios/i4003-invalid-action.json"
            -P "${CMAKE_CURRENT_LIST_DIR}/verify_i4003_invalid_scenario.cmake"
    )
    add_test(
        NAME virtual-fpga-mcs4-system-monitor
        COMMAND "${CMAKE_COMMAND}"
            "-DPROGRAM=$<TARGET_FILE:mcs4-virtual-system>"
            "-DSCENARIO=${CMAKE_CURRENT_LIST_DIR}/scenarios/mcs4-system-monitor.json"
            "-DSUMMARY=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-system-summary.json"
            "-DVCD=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-system-monitor.vcd"
            "-DTRACE_FRAMES=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-system-monitor.trace.jsonl"
            -P "${CMAKE_CURRENT_LIST_DIR}/verify_mcs4_system_scenario.cmake"
    )
    add_test(
        NAME virtual-fpga-mcs4-system-invalid-budget
        COMMAND "${CMAKE_COMMAND}"
            "-DPROGRAM=$<TARGET_FILE:mcs4-virtual-system>"
            "-DSCENARIO=${CMAKE_CURRENT_LIST_DIR}/scenarios/mcs4-system-invalid-budget.json"
            -P "${CMAKE_CURRENT_LIST_DIR}/verify_mcs4_system_invalid_scenario.cmake"
    )
    add_test(
        NAME virtual-fpga-mcs4-common-stimulus
        COMMAND "${CMAKE_COMMAND}"
            "-DPROGRAM=$<TARGET_FILE:mcs4-virtual-system>"
            "-DCARGO=${CARGO_EXECUTABLE}"
            "-DPYTHON=${Python3_EXECUTABLE}"
            "-DREPOSITORY_ROOT=${REPOSITORY_ROOT}"
            "-DSCENARIO=${CMAKE_CURRENT_LIST_DIR}/scenarios/mcs4-common-nop.json"
            "-DSUMMARY=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-summary.json"
            "-DVCD=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-nop.vcd"
            "-DFPGA_TRACE=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-fpga.trace.jsonl"
            "-DBEHAVIORAL_TRACE=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-behavioral.trace.jsonl"
            "-DCOMPARISON_REPORT=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-comparison.json"
            "-DCOMPARISON_MAX_MISMATCH_OBSERVATIONS=0"
            -P "${CMAKE_CURRENT_LIST_DIR}/verify_mcs4_common_stimulus.cmake"
    )
    add_test(
        NAME virtual-fpga-mcs4-common-control
        COMMAND "${CMAKE_COMMAND}"
            "-DPROGRAM=$<TARGET_FILE:mcs4-virtual-system>"
            "-DCARGO=${CARGO_EXECUTABLE}"
            "-DPYTHON=${Python3_EXECUTABLE}"
            "-DREPOSITORY_ROOT=${REPOSITORY_ROOT}"
            "-DSCENARIO=${CMAKE_CURRENT_LIST_DIR}/scenarios/mcs4-common-control.json"
            "-DSUMMARY=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-control-summary.json"
            "-DVCD=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-control.vcd"
            "-DFPGA_TRACE=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-control-fpga.trace.jsonl"
            "-DBEHAVIORAL_TRACE=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-control-behavioral.trace.jsonl"
            "-DCOMPARISON_REPORT=${VIRTUAL_FPGA_TEST_DIRECTORY}/mcs4-common-control-comparison.json"
            "-DCOMPARISON_MAX_MISMATCH_OBSERVATIONS=0"
            "-DREQUIRED_ACTIVE_BEHAVIORAL_SIGNAL=mcs4.control.ram"
            "-DREQUIRED_ACTIVE_FPGA_SIGNAL=mcs4.control.ram"
            -P "${CMAKE_CURRENT_LIST_DIR}/verify_mcs4_common_stimulus.cmake"
    )
    set_tests_properties(
        virtual-fpga-mcs4-system-monitor
        PROPERTIES WORKING_DIRECTORY "${VIRTUAL_FPGA_TEST_DIRECTORY}"
    )
endif()
