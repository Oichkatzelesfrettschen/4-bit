#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QCryptographicHash>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QSaveFile>
#include <QStringList>
#include <QTextStream>
#include <QTemporaryFile>

#include <cstdint>
#include <functional>
#include <limits>
#include <memory>
#include <stdexcept>

#include "Vmcs4_system_sim_top.h"
#include "trace_frame_writer.h"
#include "verilated.h"
#include "verilated_vcd_c.h"

namespace {

constexpr std::uint64_t kMaximumSystemCycles = 10'000'000;
constexpr std::uint64_t kMaximumScenarioActions = 100'000;
constexpr std::uint64_t kMaximumCommonStimulusPhases = 1'000'000;
constexpr std::uint64_t kMaximumVcdSystemCycles = 1'000'000;
constexpr std::uint64_t kResetSystemCycles = 9;
constexpr qint64 kMaximumScenarioBytes = 8 * 1024 * 1024;

class SystemBoard final {
public:
  SystemBoard() : model_(std::make_unique<Vmcs4_system_sim_top>()) {
    model_->sys_clk = 0;
    model_->rst = 0;
    model_->test_in = 0;
    model_->uart_rx = 1;
    evaluate();
  }

  ~SystemBoard() {
    closeTrace();
    model_->final();
  }

  SystemBoard(const SystemBoard &) = delete;
  SystemBoard &operator=(const SystemBoard &) = delete;

  void reset() {
    model_->rst = 1;
    model_->test_in = 0;
    model_->uart_rx = 1;
    evaluate();
    runSystemCyclesUnchecked(8, {});
    model_->rst = 0;
    runSystemCyclesUnchecked(1, {});
    system_cycles_ = 0;
    wmp_strobes_ = 0;
    saw_nonidle_bus_ = false;
    saw_phi_overlap_ = false;
    previous_wmp_strobe_ = false;
  }

  void setTestIn(bool value) {
    model_->test_in = value ? 1 : 0;
    evaluate();
  }

  void setUartRx(bool value) {
    model_->uart_rx = value ? 1 : 0;
    evaluate();
  }

  void runSystemCycles(std::uint64_t cycles,
                       const std::function<void()> &on_phase_advance) {
    if (cycles > kMaximumSystemCycles) {
      throw std::runtime_error(
          "requested system cycles exceed the board safety limit");
    }
    runSystemCyclesUnchecked(cycles, on_phase_advance);
  }

  std::uint64_t runPhases(
      std::uint64_t phases, std::uint64_t cycle_budget,
      const std::function<void()> &record_phase_frame) {
    std::uint64_t completed_phases = 0;
    std::uint64_t consumed_cycles = 0;
    while (completed_phases < phases) {
      if (consumed_cycles == cycle_budget) {
        throw std::runtime_error(
            "common stimulus phase request exceeds the system-cycle safety limit");
      }
      runSystemCyclesUnchecked(1, [&]() {
        ++completed_phases;
        record_phase_frame();
      });
      ++consumed_cycles;
    }
    return consumed_cycles;
  }

  void openTrace(const QString &path) {
    if (path.isEmpty()) {
      throw std::runtime_error("VCD path is empty");
    }
    const QFileInfo file_info(path);
    if (!QDir().mkpath(file_info.absolutePath())) {
      throw std::runtime_error("cannot create the VCD output directory");
    }

    closeTrace();
    Verilated::traceEverOn(true);
    trace_ = std::make_unique<VerilatedVcdC>();
    model_->trace(trace_.get(), 99);
    const QByteArray encoded_path = QFile::encodeName(path);
    trace_->open(encoded_path.constData());
    last_trace_time_ = std::numeric_limits<std::uint64_t>::max();
    vcd_system_cycles_ = 0;
    evaluate();
  }

  void closeTrace() {
    if (trace_ != nullptr) {
      trace_->flush();
      trace_->close();
      trace_.reset();
      last_trace_time_ = std::numeric_limits<std::uint64_t>::max();
      vcd_system_cycles_ = 0;
    }
  }

  [[nodiscard]] QJsonObject summary() const {
    return QJsonObject{
        {QStringLiteral("schema_version"), 1},
        {QStringLiteral("module"), QStringLiteral("mcs4_system_sim_top")},
        {QStringLiteral("system_cycles"), static_cast<qint64>(system_cycles_)},
        {QStringLiteral("simulation_time_ticks"),
         static_cast<qint64>(simulation_time_)},
        {QStringLiteral("wmp_strobes"), static_cast<qint64>(wmp_strobes_)},
        {QStringLiteral("saw_nonidle_bus"), saw_nonidle_bus_},
        {QStringLiteral("saw_phi_overlap"), saw_phi_overlap_},
        {QStringLiteral("saw_bus_contention"), saw_bus_contention_},
        {QStringLiteral("bus_data"), static_cast<int>(model_->debug_bus_data)},
        {QStringLiteral("bus_driven"), model_->debug_bus_driven != 0},
        {QStringLiteral("bus_producer_count"), busProducerCount()},
        {QStringLiteral("cpu_pc"), static_cast<int>(model_->debug_cpu_pc)},
        {QStringLiteral("cpu_accumulator"),
         static_cast<int>(model_->debug_cpu_accumulator)},
        {QStringLiteral("cpu_carry"), model_->debug_cpu_carry != 0},
        {QStringLiteral("cpu_phase"),
         static_cast<int>(model_->debug_cpu_phase)},
        {QStringLiteral("cm_rom"), model_->debug_cm_rom != 0},
        {QStringLiteral("cm_ram"), model_->debug_cm_ram != 0},
        {QStringLiteral("rom_selected"), model_->debug_rom_selected != 0},
        {QStringLiteral("ram_selected"), model_->debug_ram_selected != 0},
        {QStringLiteral("uart_tx"), model_->uart_tx != 0},
        {QStringLiteral("uart_rx_ready"), model_->debug_uart_rx_ready != 0},
    };
  }

  [[nodiscard]] QJsonObject traceFrame(std::uint64_t run_id,
                                       std::uint64_t sequence,
                                       std::uint64_t input_event_id,
                                       const QString &stimulus_sha256) const {
    const int producer_count = busProducerCount();
    // The HDL phase register advances on phi1 before phi2 commits the phase
    // action.  The behavioral trace labels the resulting state with the
    // completed phase, so convert the next-phase register back here.
    const std::uint8_t completed_phase =
        model_->debug_cpu_phase == 0 ? 7 : model_->debug_cpu_phase - 1;
    return QJsonObject{
        {QStringLiteral("schema_version"), 1},
        {QStringLiteral("run_id"), static_cast<qint64>(run_id)},
        {QStringLiteral("sequence"), static_cast<qint64>(sequence)},
        {QStringLiteral("input_event_id"), static_cast<qint64>(input_event_id)},
        {QStringLiteral("logical_tick"), static_cast<qint64>(system_cycles_)},
        {QStringLiteral("physical_time_ps"), QJsonValue(QJsonValue::Null)},
        {QStringLiteral("phase"), QJsonValue(QJsonValue::Null)},
        {QStringLiteral("provenance"),
         QJsonObject{
             {QStringLiteral("backend"), QStringLiteral("verilator")},
             {QStringLiteral("fidelity"), QStringLiteral("fpga")},
             {QStringLiteral("model_id"),
              QStringLiteral("mcs4-system-fpga-verilator")},
             {QStringLiteral("model_sha256"), QJsonValue(QJsonValue::Null)},
             {QStringLiteral("stimulus_sha256"), stimulus_sha256},
             {QStringLiteral("stimulus_kind"), QStringLiteral("scenario-json")},
             {QStringLiteral("evidence_status"),
              QStringLiteral("local-unsealed")},
         }},
        {QStringLiteral("signals"),
         QJsonArray{
             bitsSignal(QStringLiteral("mcs4.phase"), 3,
                        completed_phase),
             bitsSignal(QStringLiteral("mcs4.bus"), 4, model_->debug_bus_data),
             logicSignal(QStringLiteral("mcs4.bus.valid"), producer_count <= 1),
             logicSignal(QStringLiteral("mcs4.bus.contention"),
                         producer_count > 1),
             bitsSignal(QStringLiteral("mcs4.cpu.pc"), 12,
                        model_->debug_cpu_pc),
             bitsSignal(QStringLiteral("mcs4.cpu.accumulator"), 4,
                        model_->debug_cpu_accumulator),
             logicSignal(QStringLiteral("mcs4.cpu.carry"),
                         model_->debug_cpu_carry != 0),
             logicSignal(QStringLiteral("mcs4.control.rom"),
                         model_->debug_rom_selected != 0),
             logicSignal(QStringLiteral("mcs4.control.ram"),
                         model_->debug_cm_ram != 0),
             logicSignal(QStringLiteral("mcs4.fpga.bus_driven"),
                         model_->debug_bus_driven != 0),
             logicSignal(QStringLiteral("mcs4.fpga.rom_selected"),
                         model_->debug_rom_selected != 0),
             logicSignal(QStringLiteral("mcs4.fpga.ram_selected"),
                         model_->debug_ram_selected != 0),
             logicSignal(QStringLiteral("mcs4.fpga.phi1"),
                         model_->debug_phi1 != 0),
             logicSignal(QStringLiteral("mcs4.fpga.phi2"),
                         model_->debug_phi2 != 0),
             logicSignal(QStringLiteral("mcs4.fpga.cpu_data_oe"),
                         model_->debug_cpu_data_oe != 0),
             logicSignal(QStringLiteral("mcs4.fpga.rom_data_oe"),
                         model_->debug_rom_data_oe != 0),
             logicSignal(QStringLiteral("mcs4.fpga.ram_data_oe"),
                         model_->debug_ram_data_oe != 0),
             logicSignal(QStringLiteral("mcs4.fpga.wmp_strobe"),
                         model_->debug_wmp_strobe != 0),
             bitsSignal(QStringLiteral("mcs4.fpga.wmp_data"), 4,
                        model_->debug_wmp_data),
         }},
    };
  }

private:
  static QJsonObject logicSignal(const QString &path, bool value) {
    return QJsonObject{
        {QStringLiteral("path"), path},
        {QStringLiteral("value"),
         QJsonObject{
             {QStringLiteral("kind"), QStringLiteral("logic")},
             {QStringLiteral("value"),
              value ? QStringLiteral("one") : QStringLiteral("zero")},
         }},
        {QStringLiteral("source"), QStringLiteral("mcs4_system_sim_top")},
    };
  }

  static QJsonObject bitsSignal(const QString &path, int width,
                                std::uint64_t value) {
    return QJsonObject{
        {QStringLiteral("path"), path},
        {QStringLiteral("value"),
         QJsonObject{
             {QStringLiteral("kind"), QStringLiteral("bits")},
             {QStringLiteral("width"), width},
             {QStringLiteral("value"), static_cast<qint64>(value)},
         }},
        {QStringLiteral("source"), QStringLiteral("mcs4_system_sim_top")},
    };
  }

  [[nodiscard]] int busProducerCount() const {
    return static_cast<int>(model_->debug_cpu_data_oe != 0) +
           static_cast<int>(model_->debug_rom_data_oe != 0) +
           static_cast<int>(model_->debug_ram_data_oe != 0);
  }

  void evaluate() {
    model_->eval();
    if (model_->debug_phi1 != 0 && model_->debug_phi2 != 0) {
      saw_phi_overlap_ = true;
    }
    if (model_->rst == 0) {
      if (busProducerCount() > 1) {
        saw_bus_contention_ = true;
      }
      if (model_->debug_bus_data != 0x0fU) {
        saw_nonidle_bus_ = true;
      }
      const bool wmp_strobe = model_->debug_wmp_strobe != 0;
      if (wmp_strobe && !previous_wmp_strobe_) {
        ++wmp_strobes_;
      }
      previous_wmp_strobe_ = wmp_strobe;
    }
    if (trace_ != nullptr && last_trace_time_ != simulation_time_) {
      trace_->dump(simulation_time_);
      last_trace_time_ = simulation_time_;
    }
  }

  void runSystemCyclesUnchecked(std::uint64_t cycles,
                                const std::function<void()> &on_phase_advance) {
    if (trace_ != nullptr &&
        cycles > kMaximumVcdSystemCycles - vcd_system_cycles_) {
      throw std::runtime_error("VCD output exceeds the simulation-cycle safety limit");
    }
    for (std::uint64_t index = 0; index < cycles; ++index) {
      const std::uint8_t phase_before = model_->debug_cpu_phase;
      model_->sys_clk = 0;
      evaluate();
      ++simulation_time_;
      model_->sys_clk = 1;
      evaluate();
      ++simulation_time_;
      model_->sys_clk = 0;
      evaluate();
      ++simulation_time_;
      ++system_cycles_;
      if (trace_ != nullptr) {
        ++vcd_system_cycles_;
      }
      if (model_->rst == 0 && model_->debug_cpu_phase != phase_before &&
          on_phase_advance) {
        on_phase_advance();
      }
    }
  }

  std::unique_ptr<Vmcs4_system_sim_top> model_;
  std::unique_ptr<VerilatedVcdC> trace_;
  std::uint64_t simulation_time_ = 0;
  std::uint64_t system_cycles_ = 0;
  std::uint64_t wmp_strobes_ = 0;
  std::uint64_t last_trace_time_ = std::numeric_limits<std::uint64_t>::max();
  std::uint64_t vcd_system_cycles_ = 0;
  bool saw_nonidle_bus_ = false;
  bool saw_phi_overlap_ = false;
  bool saw_bus_contention_ = false;
  bool previous_wmp_strobe_ = false;
};

[[nodiscard]] bool scenarioBit(const QJsonObject &action, const QString &field,
                               int action_index) {
  const QJsonValue value = action.value(field);
  if (!value.isDouble()) {
    throw std::runtime_error(QStringLiteral("action %1 requires numeric %2")
                                 .arg(action_index)
                                 .arg(field)
                                 .toStdString());
  }
  const int integer = value.toInt(-1);
  if (integer != 0 && integer != 1) {
    throw std::runtime_error(QStringLiteral("action %1 %2 must be 0 or 1")
                                 .arg(action_index)
                                 .arg(field)
                                 .toStdString());
  }
  return integer == 1;
}

[[nodiscard]] bool scenarioBoolean(const QJsonObject &action,
                                   const QString &field, int action_index) {
  const QJsonValue value = action.value(field);
  if (!value.isBool()) {
    throw std::runtime_error(QStringLiteral("action %1 requires boolean %2")
                                 .arg(action_index)
                                 .arg(field)
                                 .toStdString());
  }
  return value.toBool();
}

[[nodiscard]] std::uint64_t scenarioCycles(const QJsonObject &action,
                                           int action_index) {
  const QJsonValue value = action.value(QStringLiteral("value"));
  if (!value.isDouble()) {
    throw std::runtime_error(QStringLiteral("action %1 requires numeric value")
                                 .arg(action_index)
                                 .toStdString());
  }
  const qint64 cycles = value.toInteger(-1);
  if (cycles < 0 || static_cast<std::uint64_t>(cycles) > kMaximumSystemCycles) {
    throw std::runtime_error(
        QStringLiteral(
            "action %1 system-cycle value is outside the board safety limit")
            .arg(action_index)
            .toStdString());
  }
  return static_cast<std::uint64_t>(cycles);
}

[[nodiscard]] QByteArray commonStimulusRom(const QJsonObject &scenario) {
  const QJsonValue value = scenario.value(QStringLiteral("rom_hex"));
  if (!value.isString()) {
    throw std::runtime_error("common stimulus rom_hex must be a string");
  }
  QString digits;
  const QString source = value.toString();
  for (const QChar character : source) {
    if (character.isSpace()) {
      continue;
    }
    const ushort code_unit = character.unicode();
    const bool is_digit = code_unit >= '0' && code_unit <= '9';
    const bool is_lower = code_unit >= 'a' && code_unit <= 'f';
    const bool is_upper = code_unit >= 'A' && code_unit <= 'F';
    if (!is_digit && !is_lower && !is_upper) {
      throw std::runtime_error("common stimulus rom_hex contains a non-hexadecimal byte");
    }
    digits.append(character);
  }
  if (digits.size() != 512) {
    throw std::runtime_error(
        "common stimulus rom_hex must contain exactly 256 hexadecimal bytes");
  }
  const QByteArray rom = QByteArray::fromHex(digits.toLatin1());
  if (rom.size() != 256) {
    throw std::runtime_error("common stimulus ROM decoding failed");
  }
  return rom;
}

void writeCommonStimulusRom(QTemporaryFile &file, const QByteArray &rom) {
  for (const unsigned char byte : rom) {
    const QByteArray line = QByteArray::number(byte, 16).rightJustified(2, '0') + '\n';
    if (file.write(line) != line.size()) {
      throw std::runtime_error("write common stimulus temporary ROM");
    }
  }
  if (!file.flush()) {
    throw std::runtime_error("flush common stimulus temporary ROM");
  }
}

void verifyExpectation(const QJsonObject &expected, const SystemBoard &board) {
  const QJsonObject observed = board.summary();
  for (const QString &field : expected.keys()) {
    if (field == QStringLiteral("min_wmp_strobes")) {
      const qint64 minimum = expected.value(field).toInteger(-1);
      if (minimum < 0 ||
          observed.value(QStringLiteral("wmp_strobes")).toInteger() < minimum) {
        throw std::runtime_error("minimum WMP-strobe expectation failed");
      }
    } else if (field == QStringLiteral("require_nonoverlap_phi")) {
      if (!expected.value(field).isBool()) {
        throw std::runtime_error("require_nonoverlap_phi must be boolean");
      }
      if (expected.value(field).toBool() &&
          observed.value(QStringLiteral("saw_phi_overlap")).toBool()) {
        throw std::runtime_error("phi1 and phi2 overlapped");
      }
    } else if (field == QStringLiteral("require_nonidle_bus")) {
      if (!expected.value(field).isBool()) {
        throw std::runtime_error("require_nonidle_bus must be boolean");
      }
      if (expected.value(field).toBool() &&
          !observed.value(QStringLiteral("saw_nonidle_bus")).toBool()) {
        throw std::runtime_error(
            "system bus remained at the inactive pull-up value");
      }
    } else if (field == QStringLiteral("require_no_bus_contention")) {
      if (!expected.value(field).isBool()) {
        throw std::runtime_error("require_no_bus_contention must be boolean");
      }
      if (expected.value(field).toBool() &&
          observed.value(QStringLiteral("saw_bus_contention")).toBool()) {
        throw std::runtime_error("system data bus had overlapping producers");
      }
    } else {
      throw std::runtime_error(QStringLiteral("unknown expected field: %1")
                                   .arg(field)
                                   .toStdString());
    }
  }
}

void writeSummary(const QString &path, const QJsonObject &summary) {
  if (path.isEmpty()) {
    return;
  }
  const QFileInfo file_info(path);
  if (!QDir().mkpath(file_info.absolutePath())) {
    throw std::runtime_error("cannot create the summary output directory");
  }
  QSaveFile output(path);
  if (!output.open(QIODevice::WriteOnly)) {
    throw std::runtime_error("cannot open the summary output");
  }
  const QByteArray bytes =
      QJsonDocument(summary).toJson(QJsonDocument::Indented);
  if (output.write(bytes) != bytes.size() || !output.commit()) {
    throw std::runtime_error("cannot commit the summary output");
  }
}

int runHeadless(const QStringList &arguments) {
  QCommandLineParser parser;
  parser.setApplicationDescription(
      QStringLiteral("Headless MCS-4 FPGA system board"));
  const QCommandLineOption help_option = parser.addHelpOption();
  const QCommandLineOption headless_option(QStringLiteral("headless"));
  const QCommandLineOption scenario_option(
      QStringLiteral("scenario"), QStringLiteral("JSON action scenario"),
      QStringLiteral("path"));
  const QCommandLineOption vcd_option(QStringLiteral("vcd"),
                                      QStringLiteral("VCD output path"),
                                      QStringLiteral("path"));
  const QCommandLineOption summary_option(
      QStringLiteral("summary"), QStringLiteral("JSON summary output path"),
      QStringLiteral("path"));
  const QCommandLineOption trace_frames_option(
      QStringLiteral("trace-frames"),
      QStringLiteral("newline-delimited shared trace-frame output path"),
      QStringLiteral("path"));
  parser.addOption(headless_option);
  parser.addOption(scenario_option);
  parser.addOption(vcd_option);
  parser.addOption(summary_option);
  parser.addOption(trace_frames_option);
  if (!parser.parse(arguments)) {
    throw std::runtime_error(parser.errorText().toStdString());
  }
  if (parser.isSet(help_option)) {
    QTextStream(stdout) << parser.helpText();
    return EXIT_SUCCESS;
  }
  if (!parser.isSet(headless_option) || !parser.isSet(scenario_option)) {
    throw std::runtime_error(
        "--headless and --scenario are required for headless execution");
  }

  QFile scenario_file(parser.value(scenario_option));
  if (!scenario_file.open(QIODevice::ReadOnly)) {
    throw std::runtime_error("cannot open the scenario file");
  }
  if (scenario_file.size() > kMaximumScenarioBytes) {
    throw std::runtime_error("scenario JSON exceeds the byte safety limit");
  }
  const QByteArray scenario_bytes = scenario_file.readAll();
  QJsonParseError parse_error;
  const QJsonDocument scenario_document =
      QJsonDocument::fromJson(scenario_bytes, &parse_error);
  if (parse_error.error != QJsonParseError::NoError ||
      !scenario_document.isObject()) {
    throw std::runtime_error(QStringLiteral("scenario JSON parse failure: %1")
                                 .arg(parse_error.errorString())
                                 .toStdString());
  }
  const QJsonObject scenario = scenario_document.object();
  if (scenario.value(QStringLiteral("schema_version")).toInt() != 1) {
    throw std::runtime_error("unsupported scenario schema version");
  }
  const QString target = scenario.value(QStringLiteral("target")).toString();
  const bool common_stimulus = target == QStringLiteral("mcs4-common-stimulus");
  if (!common_stimulus && target != QStringLiteral("mcs4-system-verilator")) {
    throw std::runtime_error(
        "scenario target must be mcs4-system-verilator or mcs4-common-stimulus");
  }
  const QJsonValue actions_value = scenario.value(QStringLiteral("actions"));
  if (!actions_value.isArray()) {
    throw std::runtime_error("scenario actions must be an array");
  }

  const QString stimulus_sha256 = QString::fromLatin1(
      QCryptographicHash::hash(scenario_bytes, QCryptographicHash::Sha256)
          .toHex());
  const QJsonArray actions = actions_value.toArray();
  if (actions.size() > static_cast<qsizetype>(kMaximumScenarioActions)) {
    throw std::runtime_error("scenario exceeds the action safety limit");
  }
  QByteArray common_rom_plusarg;
  std::unique_ptr<QTemporaryFile> common_rom_file;
  if (common_stimulus) {
    if (!scenario.value(QStringLiteral("expect")).isUndefined()) {
      throw std::runtime_error("common stimulus does not support expect");
    }
    if (actions.isEmpty()) {
      throw std::runtime_error("common stimulus requires at least one action");
    }
    const QJsonObject first_action = actions.first().toObject();
    if (first_action.value(QStringLiteral("op")).toString() !=
        QStringLiteral("reset")) {
      throw std::runtime_error("common stimulus must begin with reset");
    }
    const QByteArray rom = commonStimulusRom(scenario);
    common_rom_file = std::make_unique<QTemporaryFile>(
        QDir::tempPath() + QStringLiteral("/mcs4-common-stimulus-XXXXXX.hex"));
    common_rom_file->setAutoRemove(true);
    if (!common_rom_file->open()) {
      throw std::runtime_error("cannot create common stimulus temporary ROM");
    }
    writeCommonStimulusRom(*common_rom_file, rom);
    common_rom_file->close();
    common_rom_plusarg =
        QByteArray("+mcs4_rom_init=") + QFile::encodeName(common_rom_file->fileName());
    const char *verilator_arguments[] = {common_rom_plusarg.constData()};
    Verilated::commandArgsAdd(1, verilator_arguments);
    if (QString::fromLocal8Bit(
            Verilated::commandArgsPlusMatch("mcs4_rom_init="))
            .isEmpty()) {
      throw std::runtime_error("cannot register common stimulus ROM with Verilator");
    }
  }
  SystemBoard board;
  if (parser.isSet(vcd_option)) {
    board.openTrace(parser.value(vcd_option));
  }
  mcs4::virtual_fpga::TraceFrameWriter trace_writer(
      parser.value(trace_frames_option));
  std::uint64_t requested_system_cycles = 0;
  std::uint64_t run_id = 1;
  std::uint64_t sequence = 0;
  const auto reserve_system_cycles = [&](std::uint64_t cycles) {
    if (cycles > kMaximumSystemCycles - requested_system_cycles) {
      throw std::runtime_error(
          "scenario cumulative system cycles exceed the board safety limit");
    }
    requested_system_cycles += cycles;
  };
  for (qsizetype index = 0; index < actions.size(); ++index) {
    if (!actions.at(index).isObject()) {
      throw std::runtime_error(QStringLiteral("action %1 is not an object")
                                   .arg(index)
                                   .toStdString());
    }
    const QJsonObject action = actions.at(index).toObject();
    const QString operation = action.value(QStringLiteral("op")).toString();
    const std::uint64_t input_event_id = static_cast<std::uint64_t>(index) + 1;
    bool emitted_frame = false;
    const auto append_frame = [&]() {
      trace_writer.record([&]() {
        return board.traceFrame(run_id, ++sequence, input_event_id,
                                stimulus_sha256);
      });
      emitted_frame = true;
    };

    if (common_stimulus && operation == QStringLiteral("reset")) {
      reserve_system_cycles(kResetSystemCycles);
      board.reset();
      ++run_id;
      sequence = 0;
    } else if (common_stimulus && operation == QStringLiteral("set_test")) {
      board.setTestIn(scenarioBoolean(action, QStringLiteral("value"),
                                      static_cast<int>(index)));
    } else if (common_stimulus && operation == QStringLiteral("run_phases")) {
      const QJsonValue value = action.value(QStringLiteral("value"));
      if (!value.isDouble()) {
        throw std::runtime_error(QStringLiteral("action %1 requires numeric value")
                                     .arg(index)
                                     .toStdString());
      }
      const qint64 phases = value.toInteger(-1);
      if (phases <= 0 || static_cast<std::uint64_t>(phases) >
                             kMaximumCommonStimulusPhases) {
        throw std::runtime_error(
            "common stimulus run_phases value is outside the safety limit");
      }
      const std::uint64_t remaining_cycles =
          kMaximumSystemCycles - requested_system_cycles;
      const auto append_phase_frame = [&]() {
        trace_writer.record([&]() {
          return board.traceFrame(run_id, ++sequence, input_event_id,
                                  stimulus_sha256);
        });
        emitted_frame = true;
      };
      const std::uint64_t consumed_cycles = board.runPhases(
          static_cast<std::uint64_t>(phases), remaining_cycles,
          append_phase_frame);
      requested_system_cycles += consumed_cycles;
    } else if (!common_stimulus && operation == QStringLiteral("reset")) {
      reserve_system_cycles(kResetSystemCycles);
      board.reset();
      ++run_id;
      sequence = 0;
    } else if (!common_stimulus && operation == QStringLiteral("set_test_in")) {
      board.setTestIn(scenarioBit(action, QStringLiteral("value"),
                                  static_cast<int>(index)));
    } else if (!common_stimulus && operation == QStringLiteral("set_uart_rx")) {
      board.setUartRx(scenarioBit(action, QStringLiteral("value"),
                                  static_cast<int>(index)));
    } else if (!common_stimulus && operation == QStringLiteral("run_sys_cycles")) {
      const std::uint64_t cycles =
          scenarioCycles(action, static_cast<int>(index));
      reserve_system_cycles(cycles);
      board.runSystemCycles(cycles, append_frame);
    } else {
      throw std::runtime_error(QStringLiteral("action %1 has unknown op %2")
                                   .arg(index)
                                   .arg(operation)
                                   .toStdString());
    }
    if (!common_stimulus && !emitted_frame) {
      append_frame();
    }
  }

  const QJsonValue expected_value = scenario.value(QStringLiteral("expect"));
  if (!common_stimulus && !expected_value.isUndefined()) {
    if (!expected_value.isObject()) {
      throw std::runtime_error("scenario expect must be an object");
    }
    verifyExpectation(expected_value.toObject(), board);
  }

  QJsonObject summary = board.summary();
  summary.insert(QStringLiteral("trace_frame_count"),
                 static_cast<qint64>(trace_writer.frameCount()));
  writeSummary(parser.value(summary_option), summary);
  trace_writer.commit();
  QTextStream(stdout) << QJsonDocument(summary).toJson(QJsonDocument::Compact)
                      << '\n';
  return EXIT_SUCCESS;
}

} // namespace

int main(int argc, char *argv[]) {
  try {
    Verilated::commandArgs(argc, argv);
    QCoreApplication application(argc, argv);
    return runHeadless(application.arguments());
  } catch (const std::exception &error) {
    QTextStream(stderr) << "mcs4-virtual-system: " << error.what() << '\n';
    return EXIT_FAILURE;
  }
}
