#pragma once

#include "desktop/kernel/error_codes.h"

#include <QString>
#include <utility>
#include <variant>

namespace desktop::kernel {

// A value or an error, never an exception: errors cross layer boundaries as
// values (playbook 3.2).
template <class T>
class [[nodiscard]] Result {
public:
    Result(T value) : data_(std::move(value)) {}

    Result(ErrorCode code, QString message = QString())
        : data_(Error{code, std::move(message)}) {}

    [[nodiscard]] bool ok() const noexcept { return std::holds_alternative<T>(data_); }
    explicit operator bool() const noexcept { return ok(); }

    // Precondition: ok().
    [[nodiscard]] T& value() { return std::get<T>(data_); }
    [[nodiscard]] const T& value() const { return std::get<T>(data_); }

    [[nodiscard]] ErrorCode errorCode() const noexcept {
        return ok() ? ErrorCode::Unknown : std::get<Error>(data_).code;
    }

    [[nodiscard]] const QString& errorMessage() const noexcept {
        static const QString kEmpty;
        return ok() ? kEmpty : std::get<Error>(data_).message;
    }

    [[nodiscard]] T valueOr(T fallback) const {
        return ok() ? value() : std::move(fallback);
    }

private:
    struct Error {
        ErrorCode code = ErrorCode::Unknown;
        QString message;
    };

    std::variant<T, Error> data_;
};

// The specialization carries no value, only the outcome.
template <>
class Result<void> {
public:
    Result() = default;

    Result(ErrorCode code, QString message = QString())
        : ok_(false), code_(code), message_(std::move(message)) {}

    [[nodiscard]] bool ok() const noexcept { return ok_; }
    explicit operator bool() const noexcept { return ok_; }

    [[nodiscard]] ErrorCode errorCode() const noexcept {
        return ok_ ? ErrorCode::Unknown : code_;
    }

    [[nodiscard]] const QString& errorMessage() const noexcept { return message_; }

private:
    bool ok_ = true;
    ErrorCode code_ = ErrorCode::Unknown;
    QString message_;
};

}  // namespace desktop::kernel
