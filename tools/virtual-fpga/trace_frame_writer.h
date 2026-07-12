#pragma once

#include <QDir>
#include <QFileInfo>
#include <QIODevice>
#include <QJsonDocument>
#include <QJsonObject>
#include <QSaveFile>
#include <QString>

#include <cstdint>
#include <functional>
#include <memory>
#include <stdexcept>

namespace mcs4::virtual_fpga {

inline constexpr std::uint64_t kMaximumTraceFrames = 100'000;
inline constexpr std::uint64_t kMaximumTraceFrameBytes = 64U * 1024U * 1024U;

/// Atomically stream bounded JSONL trace frames without retaining an in-memory array.
class TraceFrameWriter final {
public:
  explicit TraceFrameWriter(const QString &path) {
    if (path.isEmpty()) {
      return;
    }
    const QFileInfo file_info(path);
    if (!QDir().mkpath(file_info.absolutePath())) {
      throw std::runtime_error("cannot create the trace-frame output directory");
    }
    output_ = std::make_unique<QSaveFile>(path);
    if (!output_->open(QIODevice::WriteOnly)) {
      throw std::runtime_error("cannot open the trace-frame output");
    }
  }

  TraceFrameWriter(const TraceFrameWriter &) = delete;
  TraceFrameWriter &operator=(const TraceFrameWriter &) = delete;

  void record(const std::function<QJsonObject()> &build_frame) {
    if (frame_count_ >= kMaximumTraceFrames) {
      throw std::runtime_error("scenario exceeds the trace-frame safety limit");
    }
    ++frame_count_;
    if (output_ == nullptr) {
      return;
    }

    const QByteArray line =
        QJsonDocument(build_frame()).toJson(QJsonDocument::Compact);
    const std::uint64_t encoded_bytes =
        static_cast<std::uint64_t>(line.size()) + 1U;
    if (encoded_bytes > kMaximumTraceFrameBytes ||
        serialized_bytes_ > kMaximumTraceFrameBytes - encoded_bytes) {
      throw std::runtime_error("trace-frame output exceeds the byte safety limit");
    }
    if (output_->write(line) != line.size() || output_->write("\n") != 1) {
      throw std::runtime_error("cannot write the trace-frame output");
    }
    serialized_bytes_ += encoded_bytes;
  }

  void commit() {
    if (output_ != nullptr && !output_->commit()) {
      throw std::runtime_error("cannot commit the trace-frame output");
    }
  }

  [[nodiscard]] std::uint64_t frameCount() const { return frame_count_; }

private:
  std::unique_ptr<QSaveFile> output_;
  std::uint64_t frame_count_ = 0;
  std::uint64_t serialized_bytes_ = 0;
};

} // namespace mcs4::virtual_fpga
