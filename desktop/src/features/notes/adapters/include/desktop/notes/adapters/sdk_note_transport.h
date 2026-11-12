#pragma once

#include "desktop/kernel/result.h"
#include "desktop/notes/domain/note.h"
#include "desktop/sdk/notes_client.h"

#include <QVector>
#include <functional>

namespace desktop::notes::adapters {

// The out adapter that talks to the server through the SDK: the DTOs of the
// SDK become the notes of the domain.
class SdkNoteTransport {
public:
    using Callback = std::function<void(kernel::Result<QVector<domain::Note>>)>;

    explicit SdkNoteTransport(sdk::NotesClient& client);

    // Fetches the notes of the server and hands them to the callback.
    void fetchNotes(Callback callback);

private:
    sdk::NotesClient& client_;
};

}  // namespace desktop::notes::adapters
