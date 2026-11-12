#pragma once

#include "desktop/contracts/note_dto.h"
#include "desktop/kernel/ids.h"
#include "desktop/kernel/result.h"
#include "desktop/kernel/time.h"

#include <QString>

namespace desktop::notes::domain {

// The note entity and its invariants (playbook 3.1: the rules live in the
// domain, never in the form and never in the storage).
class Note {
public:
    static constexpr int kMaxTitleLength = 120;
    static constexpr int kMaxBodyLength = 10000;

    // A new note: the caller owns the identity and the clock.
    [[nodiscard]] static kernel::Result<Note> create(kernel::NoteId id, QString title, QString body,
                                                     kernel::Timestamp createdAt,
                                                     kernel::Timestamp updatedAt);

    // A note that arrives from the storage or from the server: the same
    // invariants apply to it.
    [[nodiscard]] static kernel::Result<Note> fromDto(const contracts::NoteDto& dto);

    [[nodiscard]] const kernel::NoteId& id() const noexcept { return id_; }
    [[nodiscard]] const QString& title() const noexcept { return title_; }
    [[nodiscard]] const QString& body() const noexcept { return body_; }
    [[nodiscard]] const kernel::Timestamp& createdAt() const noexcept { return createdAt_; }
    [[nodiscard]] const kernel::Timestamp& updatedAt() const noexcept { return updatedAt_; }

    [[nodiscard]] contracts::NoteDto toDto() const;

private:
    Note(kernel::NoteId id, QString title, QString body, kernel::Timestamp createdAt,
         kernel::Timestamp updatedAt);

    kernel::NoteId id_;
    QString title_;
    QString body_;
    kernel::Timestamp createdAt_;
    kernel::Timestamp updatedAt_;
};

}  // namespace desktop::notes::domain
