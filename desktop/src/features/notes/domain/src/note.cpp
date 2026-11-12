#include "desktop/notes/domain/note.h"

#include "desktop/kernel/error_codes.h"

#include <utility>

namespace desktop::notes::domain {
namespace {

// A note is plain text: a tag-like fragment never enters the entity.
bool containsMarkup(const QString& text) {
    for (int index = 0; index < text.size(); ++index) {
        if (text.at(index) != QLatin1Char('<')) {
            continue;
        }
        const QChar next = index + 1 < text.size() ? text.at(index + 1) : QChar();
        if (next.isLetter() || next == QLatin1Char('/') || next == QLatin1Char('!')) {
            return true;
        }
    }
    return false;
}

kernel::Result<void> validate(const QString& title, const QString& body) {
    if (title.isEmpty()) {
        return kernel::Result<void>(kernel::ErrorCode::ValidationFailed,
                                    QStringLiteral("the title is empty"));
    }
    if (title.size() > Note::kMaxTitleLength) {
        return kernel::Result<void>(
            kernel::ErrorCode::ValidationFailed,
            QStringLiteral("the title is longer than %1 characters").arg(Note::kMaxTitleLength));
    }
    if (body.size() > Note::kMaxBodyLength) {
        return kernel::Result<void>(
            kernel::ErrorCode::ValidationFailed,
            QStringLiteral("the body is longer than %1 characters").arg(Note::kMaxBodyLength));
    }
    if (containsMarkup(title) || containsMarkup(body)) {
        return kernel::Result<void>(kernel::ErrorCode::ValidationFailed,
                                    QStringLiteral("a note is plain text, not markup"));
    }
    return kernel::Result<void>();
}

}  // namespace

Note::Note(kernel::NoteId id, QString title, QString body, kernel::Timestamp createdAt,
           kernel::Timestamp updatedAt)
    : id_(std::move(id)),
      title_(std::move(title)),
      body_(std::move(body)),
      createdAt_(createdAt),
      updatedAt_(updatedAt) {}

kernel::Result<Note> Note::create(kernel::NoteId id, QString title, QString body,
                                   kernel::Timestamp createdAt, kernel::Timestamp updatedAt) {
    title = title.trimmed();
    const kernel::Result<void> valid = validate(title, body);
    if (!valid.ok()) {
        return kernel::Result<Note>(valid.errorCode(), valid.errorMessage());
    }
    return kernel::Result<Note>(
        Note(std::move(id), std::move(title), std::move(body), createdAt, updatedAt));
}

kernel::Result<Note> Note::fromDto(const contracts::NoteDto& dto) {
    return create(dto.id, dto.title, dto.body, dto.createdAt, dto.updatedAt);
}

contracts::NoteDto Note::toDto() const {
    contracts::NoteDto dto;
    dto.id = id_;
    dto.title = title_;
    dto.body = body_;
    dto.createdAt = createdAt_;
    dto.updatedAt = updatedAt_;
    return dto;
}

}  // namespace desktop::notes::domain
