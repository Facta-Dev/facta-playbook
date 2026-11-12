#pragma once

#include <QString>
#include <utility>

namespace desktop::kernel {

// A value of its own type: the id of one entity never mixes with the id of
// another, and a plain string never reaches an id parameter by accident.
template <class Tag>
class StrongId {
public:
    StrongId() = default;

    explicit StrongId(QString value) : value_(std::move(value)) {}

    [[nodiscard]] const QString& value() const noexcept { return value_; }
    [[nodiscard]] bool isValid() const { return !value_.isEmpty(); }
    [[nodiscard]] QString toString() const { return value_; }

    friend bool operator==(const StrongId& lhs, const StrongId& rhs) = default;

private:
    QString value_;
};

struct NoteIdTag {};
using NoteId = StrongId<NoteIdTag>;

}  // namespace desktop::kernel
