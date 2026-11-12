#pragma once

#include "desktop/kernel/time.h"

#include <QMap>
#include <QString>
#include <QVector>

namespace desktop::infrastructure::telemetry {

enum class Category { App, Http, Storage, Realtime, Platform };

struct LogEntry {
    kernel::Timestamp timestamp;
    Category category = Category::App;
    QString message;
    QMap<QString, QString> fields;
};

// The category logger of the app (playbook 13). The text output goes to the
// Qt logging categories; the ring buffer keeps the last entries for the
// diagnostics bundle. Secrets never reach either of them.
class Logger {
public:
    explicit Logger(int ringCapacity = 512);

    void log(Category category, const QString& message, QMap<QString, QString> fields = {});

    void setRedactionEnabled(bool enabled) { redactionEnabled_ = enabled; }

    // The entries of the ring buffer, oldest first.
    [[nodiscard]] QVector<LogEntry> entries() const { return ring_; }
    [[nodiscard]] int capacity() const { return capacity_; }

    // The name of a category, as it appears in the log.
    [[nodiscard]] static QString categoryName(Category category);

    // Masks the bearer tokens of a free text.
    [[nodiscard]] static QString redactText(const QString& text);

private:
    [[nodiscard]] static bool isSensitiveKey(const QString& key);

    int capacity_;
    bool redactionEnabled_ = true;
    QVector<LogEntry> ring_;
};

}  // namespace desktop::infrastructure::telemetry
