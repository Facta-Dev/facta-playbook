#include "desktop/contracts/note_dto.h"

namespace desktop::contracts {

QJsonObject toJson(const NoteDto& note) {
    QJsonObject object;
    object.insert(QStringLiteral("id"), note.id.value());
    object.insert(QStringLiteral("title"), note.title);
    object.insert(QStringLiteral("body"), note.body);
    object.insert(QStringLiteral("createdAt"), note.createdAt.toIso8601());
    object.insert(QStringLiteral("updatedAt"), note.updatedAt.toIso8601());
    return object;
}

NoteDto noteFromJson(const QJsonObject& object, bool* ok) {
    NoteDto note;

    const QString id = object.value(QStringLiteral("id")).toString();
    note.id = kernel::NoteId(id);
    note.title = object.value(QStringLiteral("title")).toString();
    note.body = object.value(QStringLiteral("body")).toString();

    bool timestampOk = false;
    note.createdAt =
        kernel::Timestamp::fromIso8601(object.value(QStringLiteral("createdAt")).toString(),
                                        &timestampOk);
    bool createdOk = timestampOk;
    note.updatedAt =
        kernel::Timestamp::fromIso8601(object.value(QStringLiteral("updatedAt")).toString(),
                                        &timestampOk);

    const bool parsed = !id.isEmpty() && createdOk && timestampOk;
    if (ok != nullptr) {
        *ok = parsed;
    }
    return note;
}

}  // namespace desktop::contracts
