#pragma once

#include <QDateTime>
#include <QString>

namespace desktop::kernel {

// A point in time, always UTC. The wire format is ISO-8601 with milliseconds.
class Timestamp {
public:
    Timestamp() = default;

    explicit Timestamp(QDateTime utc) : value_(utc.toUTC()) {}

    // The current time, from the system clock.
    [[nodiscard]] static Timestamp now();

    // Parses an ISO-8601 timestamp; an unparsable text yields an invalid
    // Timestamp and sets *ok to false when ok is given.
    [[nodiscard]] static Timestamp fromIso8601(const QString& text, bool* ok = nullptr);

    [[nodiscard]] const QDateTime& toQDateTime() const noexcept { return value_; }
    [[nodiscard]] QString toIso8601() const;
    [[nodiscard]] bool isValid() const { return value_.isValid(); }

    friend bool operator==(const Timestamp& lhs, const Timestamp& rhs) = default;

private:
    QDateTime value_;
};

}  // namespace desktop::kernel
