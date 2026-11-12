#pragma once

#include "desktop/kernel/ids.h"
#include "desktop/kernel/time.h"

#include <QJsonObject>
#include <QString>

namespace desktop::contracts {

// The wire shape of a note. The domain owns the entity; this is the shape
// that crosses the network and the storage boundary.
struct NoteDto {
    kernel::NoteId id;
    QString title;
    QString body;
    kernel::Timestamp createdAt;
    kernel::Timestamp updatedAt;
};

[[nodiscard]] QJsonObject toJson(const NoteDto& note);

// Reads a note; a missing id or an unparsable timestamp sets *ok to false
// when ok is given.
[[nodiscard]] NoteDto noteFromJson(const QJsonObject& object, bool* ok = nullptr);

}  // namespace desktop::contracts
