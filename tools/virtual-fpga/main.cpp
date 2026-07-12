#include <QApplication>
#include <QCheckBox>
#include <QCommandLineOption>
#include <QCommandLineParser>
#include <QCoreApplication>
#include <QDir>
#include <QFile>
#include <QFileInfo>
#include <QFormLayout>
#include <QFrame>
#include <QGridLayout>
#include <QGroupBox>
#include <QHBoxLayout>
#include <QJsonArray>
#include <QJsonDocument>
#include <QJsonObject>
#include <QJsonParseError>
#include <QJsonValue>
#include <QLabel>
#include <QLineEdit>
#include <QMainWindow>
#include <QMessageBox>
#include <QPushButton>
#include <QSaveFile>
#include <QSpinBox>
#include <QStringList>
#include <QTextStream>
#include <QVBoxLayout>
#include <QWidget>

#include <array>
#include <cstdint>
#include <functional>
#include <limits>
#include <memory>
#include <stdexcept>
#include <string>

#include "Vi4003_fpga.h"
#include "trace_frame_writer.h"
#include "verilated.h"
#include "verilated_vcd_c.h"

namespace {

constexpr std::uint64_t kMaximumSystemCycles = 10'000'000;
constexpr std::uint64_t kMaximumScenarioActions = 100'000;
constexpr std::uint64_t kMaximumVcdSystemCycles = 1'000'000;
constexpr std::uint64_t kResetSystemCycles = 3;
constexpr std::uint64_t kPulseCpSystemCycles = 3;
constexpr qint64 kMaximumScenarioBytes = 8 * 1024 * 1024;
constexpr int kI4003Width = 10;

class I4003Board final {
public:
  I4003Board() : model_(std::make_unique<Vi4003_fpga>()) {
    model_->sys_clk = 0;
    model_->rst = 0;
    model_->clk_in = 0;
    model_->data_in = 0;
    model_->enable_n = 1;
    evaluate();
    reset();
  }

  ~I4003Board() {
    closeTrace();
    model_->final();
  }

  I4003Board(const I4003Board &) = delete;
  I4003Board &operator=(const I4003Board &) = delete;

  void reset() {
    model_->rst = 1;
    model_->clk_in = 0;
    model_->data_in = 0;
    model_->enable_n = 1;
    evaluate();
    runSystemCyclesUnchecked(2);
    model_->rst = 0;
    runSystemCyclesUnchecked(1);
    system_cycles_ = 0;
  }

  void setData(bool value) {
    model_->data_in = value ? 1 : 0;
    evaluate();
  }

  void setEnableN(bool value) {
    model_->enable_n = value ? 1 : 0;
    evaluate();
  }

  void pulseCp() {
    model_->clk_in = 0;
    evaluate();
    runSystemCyclesUnchecked(1);
    model_->clk_in = 1;
    evaluate();
    runSystemCyclesUnchecked(1);
    model_->clk_in = 0;
    evaluate();
    runSystemCyclesUnchecked(1);
  }

  void runSystemCycles(std::uint64_t cycles) {
    if (cycles > kMaximumSystemCycles) {
      throw std::runtime_error(
          "requested system cycles exceed the board safety limit");
    }
    runSystemCyclesUnchecked(cycles);
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

  [[nodiscard]] std::uint16_t parallelOut() const {
    return static_cast<std::uint16_t>(model_->parallel_out);
  }

  [[nodiscard]] bool serialOut() const { return model_->serial_out != 0; }

  [[nodiscard]] bool dataIn() const { return model_->data_in != 0; }

  [[nodiscard]] bool enableN() const { return model_->enable_n != 0; }

  [[nodiscard]] std::uint64_t systemCycles() const { return system_cycles_; }

  [[nodiscard]] std::uint64_t simulationTime() const {
    return simulation_time_;
  }

  [[nodiscard]] QJsonObject summary() const {
    return QJsonObject{
        {QStringLiteral("schema_version"), 1},
        {QStringLiteral("module"), QStringLiteral("i4003_fpga")},
        {QStringLiteral("system_cycles"), static_cast<qint64>(system_cycles_)},
        {QStringLiteral("simulation_time_ticks"),
         static_cast<qint64>(simulation_time_)},
        {QStringLiteral("data_in"), dataIn() ? 1 : 0},
        {QStringLiteral("enable_n"), enableN() ? 1 : 0},
        {QStringLiteral("parallel_out"), static_cast<int>(parallelOut())},
        {QStringLiteral("serial_out"), serialOut() ? 1 : 0},
    };
  }

  [[nodiscard]] QJsonObject traceFrame(std::uint64_t run_id,
                                       std::uint64_t sequence,
                                       std::uint64_t input_event_id) const {
    const auto logic_signal = [](const QString &path, bool value) {
      return QJsonObject{
          {QStringLiteral("path"), path},
          {QStringLiteral("value"),
           QJsonObject{
               {QStringLiteral("kind"), QStringLiteral("logic")},
               {QStringLiteral("value"),
                value ? QStringLiteral("one") : QStringLiteral("zero")}}},
          {QStringLiteral("source"), QStringLiteral("i4003_fpga")},
      };
    };
    const auto bits_signal = [](const QString &path, int width,
                                std::uint64_t value) {
      return QJsonObject{
          {QStringLiteral("path"), path},
          {QStringLiteral("value"),
           QJsonObject{{QStringLiteral("kind"), QStringLiteral("bits")},
                       {QStringLiteral("width"), width},
                       {QStringLiteral("value"), static_cast<qint64>(value)}}},
          {QStringLiteral("source"), QStringLiteral("i4003_fpga")},
      };
    };

    return QJsonObject{
        {QStringLiteral("schema_version"), 1},
        {QStringLiteral("run_id"), static_cast<qint64>(run_id)},
        {QStringLiteral("sequence"), static_cast<qint64>(sequence)},
        {QStringLiteral("input_event_id"), static_cast<qint64>(input_event_id)},
        {QStringLiteral("logical_tick"), static_cast<qint64>(input_event_id)},
        {QStringLiteral("physical_time_ps"), QJsonValue(QJsonValue::Null)},
        {QStringLiteral("phase"), QJsonValue(QJsonValue::Null)},
        {QStringLiteral("provenance"),
         QJsonObject{
             {QStringLiteral("backend"), QStringLiteral("verilator")},
             {QStringLiteral("fidelity"), QStringLiteral("fpga")},
             {QStringLiteral("model_id"),
              QStringLiteral("i4003-fpga-verilator")},
             {QStringLiteral("model_sha256"), QJsonValue(QJsonValue::Null)},
             {QStringLiteral("stimulus_sha256"), QJsonValue(QJsonValue::Null)},
             {QStringLiteral("evidence_status"),
              QStringLiteral("local-unsealed")},
         }},
        {QStringLiteral("signals"),
         QJsonArray{
             logic_signal(QStringLiteral("i4003.clk_in"), model_->clk_in != 0),
             logic_signal(QStringLiteral("i4003.data_in"), dataIn()),
             logic_signal(QStringLiteral("i4003.enable_n"), enableN()),
             bits_signal(QStringLiteral("i4003.parallel_out"), kI4003Width,
                         parallelOut()),
             logic_signal(QStringLiteral("i4003.serial_out"), serialOut()),
             bits_signal(QStringLiteral("i4003.system_cycles"), 64,
                         systemCycles()),
         }},
    };
  }

private:
  void evaluate() {
    model_->eval();
    if (trace_ != nullptr && last_trace_time_ != simulation_time_) {
      trace_->dump(simulation_time_);
      last_trace_time_ = simulation_time_;
    }
  }

  void runSystemCyclesUnchecked(std::uint64_t cycles) {
    if (trace_ != nullptr &&
        cycles > kMaximumVcdSystemCycles - vcd_system_cycles_) {
      throw std::runtime_error("VCD output exceeds the simulation-cycle safety limit");
    }
    for (std::uint64_t index = 0; index < cycles; ++index) {
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
    }
  }

  std::unique_ptr<Vi4003_fpga> model_;
  std::unique_ptr<VerilatedVcdC> trace_;
  std::uint64_t simulation_time_ = 0;
  std::uint64_t system_cycles_ = 0;
  std::uint64_t last_trace_time_ = std::numeric_limits<std::uint64_t>::max();
  std::uint64_t vcd_system_cycles_ = 0;
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

[[nodiscard]] std::uint64_t scenarioCycles(const QJsonObject &action,
                                           int action_index) {
  const QJsonValue value = action.value(QStringLiteral("value"));
  if (!value.isDouble()) {
    throw std::runtime_error(QStringLiteral("action %1 requires numeric value")
                                 .arg(action_index)
                                 .toStdString());
  }
  const int cycles = value.toInt(-1);
  if (cycles < 0 || static_cast<std::uint64_t>(cycles) > kMaximumSystemCycles) {
    throw std::runtime_error(
        QStringLiteral(
            "action %1 system cycle value is outside the board safety limit")
            .arg(action_index)
            .toStdString());
  }
  return static_cast<std::uint64_t>(cycles);
}

void verifyExpectation(const QJsonObject &expected, const I4003Board &board) {
  const QJsonObject observed = board.summary();
  for (const QString &field : expected.keys()) {
    if (!observed.contains(field)) {
      throw std::runtime_error(QStringLiteral("unknown expected field: %1")
                                   .arg(field)
                                   .toStdString());
    }
    if (observed.value(field) != expected.value(field)) {
      throw std::runtime_error(
          QStringLiteral(
              "expectation mismatch for %1: expected %2, observed %3")
              .arg(field, expected.value(field).toVariant().toString(),
                   observed.value(field).toVariant().toString())
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
      QStringLiteral("Headless i4003 FPGA-safe virtual board"));
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
  QJsonParseError parse_error;
  const QJsonDocument scenario_document =
      QJsonDocument::fromJson(scenario_file.readAll(), &parse_error);
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
  const QJsonValue actions_value = scenario.value(QStringLiteral("actions"));
  if (!actions_value.isArray()) {
    throw std::runtime_error("scenario actions must be an array");
  }

  I4003Board board;
  if (parser.isSet(vcd_option)) {
    board.openTrace(parser.value(vcd_option));
  }
  const QJsonArray actions = actions_value.toArray();
  if (actions.size() > static_cast<qsizetype>(kMaximumScenarioActions)) {
    throw std::runtime_error("scenario exceeds the action safety limit");
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
    if (operation == QStringLiteral("reset")) {
      reserve_system_cycles(kResetSystemCycles);
      board.reset();
      ++run_id;
      sequence = 0;
    } else if (operation == QStringLiteral("set_data")) {
      board.setData(scenarioBit(action, QStringLiteral("value"),
                                static_cast<int>(index)));
    } else if (operation == QStringLiteral("set_enable_n") ||
               operation == QStringLiteral("set_e")) {
      board.setEnableN(scenarioBit(action, QStringLiteral("value"),
                                   static_cast<int>(index)));
    } else if (operation == QStringLiteral("pulse_cp")) {
      reserve_system_cycles(kPulseCpSystemCycles);
      board.pulseCp();
    } else if (operation == QStringLiteral("run_sys_cycles")) {
      const std::uint64_t cycles =
          scenarioCycles(action, static_cast<int>(index));
      reserve_system_cycles(cycles);
      board.runSystemCycles(cycles);
    } else {
      throw std::runtime_error(QStringLiteral("action %1 has unknown op %2")
                                   .arg(index)
                                   .arg(operation)
                                   .toStdString());
    }
    trace_writer.record([&]() {
      return board.traceFrame(run_id, ++sequence,
                              static_cast<std::uint64_t>(index) + 1);
    });
  }

  const QJsonValue expected_value = scenario.value(QStringLiteral("expect"));
  if (!expected_value.isUndefined()) {
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

class BoardWindow final : public QMainWindow {
public:
  BoardWindow() {
    setWindowTitle(QStringLiteral("MCS-4 Virtual FPGA Board: Intel 4003"));
    auto *central = new QWidget(this);
    auto *layout = new QVBoxLayout(central);

    auto *controls = new QGroupBox(QStringLiteral("I4003 inputs"), central);
    auto *form = new QFormLayout(controls);
    data_input_ = new QSpinBox(controls);
    data_input_->setRange(0, 1);
    form->addRow(QStringLiteral("Serial data"), data_input_);
    enable_n_ = new QCheckBox(QStringLiteral("E high: mask parallel outputs"),
                              controls);
    enable_n_->setChecked(true);
    form->addRow(QStringLiteral("Enable_n"), enable_n_);

    auto *buttons = new QHBoxLayout();
    auto *reset_button = new QPushButton(QStringLiteral("Reset"), controls);
    auto *pulse_button = new QPushButton(QStringLiteral("Pulse CP"), controls);
    auto *run_button =
        new QPushButton(QStringLiteral("Run system cycles"), controls);
    run_cycles_ = new QSpinBox(controls);
    run_cycles_->setRange(1, 1'000'000);
    run_cycles_->setValue(1);
    buttons->addWidget(reset_button);
    buttons->addWidget(pulse_button);
    buttons->addWidget(run_cycles_);
    buttons->addWidget(run_button);
    form->addRow(buttons);
    layout->addWidget(controls);

    auto *trace_controls =
        new QGroupBox(QStringLiteral("Waveform capture"), central);
    auto *trace_layout = new QHBoxLayout(trace_controls);
    trace_path_ = new QLineEdit(trace_controls);
    trace_path_->setPlaceholderText(QStringLiteral("Optional VCD path"));
    auto *trace_button =
        new QPushButton(QStringLiteral("Open VCD"), trace_controls);
    auto *close_trace_button =
        new QPushButton(QStringLiteral("Close VCD"), trace_controls);
    trace_layout->addWidget(trace_path_);
    trace_layout->addWidget(trace_button);
    trace_layout->addWidget(close_trace_button);
    layout->addWidget(trace_controls);

    auto *outputs = new QGroupBox(QStringLiteral("I4003 outputs"), central);
    auto *output_layout = new QGridLayout(outputs);
    for (int bit = 0; bit < kI4003Width; ++bit) {
      auto *name = new QLabel(QStringLiteral("Q%1").arg(bit), outputs);
      auto *led = makeLed(outputs);
      output_leds_[static_cast<std::size_t>(bit)] = led;
      output_layout->addWidget(name, bit / 5, (bit % 5) * 2);
      output_layout->addWidget(led, bit / 5, (bit % 5) * 2 + 1);
    }
    output_layout->addWidget(new QLabel(QStringLiteral("Serial"), outputs), 2,
                             0);
    serial_led_ = makeLed(outputs);
    output_layout->addWidget(serial_led_, 2, 1);
    layout->addWidget(outputs);

    status_ = new QLabel(central);
    layout->addWidget(status_);
    setCentralWidget(central);

    connect(data_input_, qOverload<int>(&QSpinBox::valueChanged), this,
            [this](int value) {
              board_.setData(value != 0);
              refresh();
            });
    connect(enable_n_, &QCheckBox::toggled, this, [this](bool value) {
      board_.setEnableN(value);
      refresh();
    });
    connect(reset_button, &QPushButton::clicked, this, [this]() {
      board_.reset();
      refresh();
    });
    connect(pulse_button, &QPushButton::clicked, this, [this]() {
      board_.pulseCp();
      refresh();
    });
    connect(run_button, &QPushButton::clicked, this, [this]() {
      execute([this]() {
        board_.runSystemCycles(
            static_cast<std::uint64_t>(run_cycles_->value()));
      });
    });
    connect(trace_button, &QPushButton::clicked, this, [this]() {
      execute([this]() { board_.openTrace(trace_path_->text()); });
    });
    connect(close_trace_button, &QPushButton::clicked, this, [this]() {
      board_.closeTrace();
      refresh();
    });

    refresh();
  }

private:
  static QLabel *makeLed(QWidget *parent) {
    auto *led = new QLabel(parent);
    led->setAlignment(Qt::AlignCenter);
    led->setMinimumWidth(42);
    return led;
  }

  void execute(const std::function<void()> &action) {
    try {
      action();
      refresh();
    } catch (const std::exception &error) {
      QMessageBox::critical(this, QStringLiteral("Virtual FPGA board error"),
                            QString::fromUtf8(error.what()));
    }
  }

  void refresh() {
    const std::uint16_t outputs = board_.parallelOut();
    for (int bit = 0; bit < kI4003Width; ++bit) {
      const bool active = ((outputs >> bit) & 1U) != 0U;
      QLabel *led = output_leds_[static_cast<std::size_t>(bit)];
      led->setText(active ? QStringLiteral("ON") : QStringLiteral("OFF"));
      led->setStyleSheet(
          active ? QStringLiteral("background: #208020; color: white;")
                 : QStringLiteral("background: #404040; color: white;"));
    }
    const bool serial_active = board_.serialOut();
    serial_led_->setText(serial_active ? QStringLiteral("ON")
                                       : QStringLiteral("OFF"));
    serial_led_->setStyleSheet(
        serial_active ? QStringLiteral("background: #208020; color: white;")
                      : QStringLiteral("background: #404040; color: white;"));
    status_->setText(QStringLiteral("System cycles: %1   Simulation ticks: %2  "
                                    " Parallel: 0x%3   Serial: %4")
                         .arg(board_.systemCycles())
                         .arg(board_.simulationTime())
                         .arg(board_.parallelOut(), 3, 16, QLatin1Char('0'))
                         .arg(board_.serialOut() ? 1 : 0));
  }

  I4003Board board_;
  QSpinBox *data_input_ = nullptr;
  QCheckBox *enable_n_ = nullptr;
  QSpinBox *run_cycles_ = nullptr;
  QLineEdit *trace_path_ = nullptr;
  std::array<QLabel *, kI4003Width> output_leds_{};
  QLabel *serial_led_ = nullptr;
  QLabel *status_ = nullptr;
};

int runGui(QApplication &application) {
  BoardWindow window;
  window.resize(720, 420);
  window.show();
  return application.exec();
}

} // namespace

int main(int argc, char *argv[]) {
  bool headless = false;
  for (int index = 1; index < argc; ++index) {
    if (QString::fromLocal8Bit(argv[index]) == QStringLiteral("--headless")) {
      headless = true;
      break;
    }
  }

  try {
    if (headless) {
      QCoreApplication application(argc, argv);
      return runHeadless(application.arguments());
    }
    QApplication application(argc, argv);
    return runGui(application);
  } catch (const std::exception &error) {
    QTextStream(stderr) << "mcs4-virtual-fpga: " << error.what() << '\n';
    return EXIT_FAILURE;
  }
}
