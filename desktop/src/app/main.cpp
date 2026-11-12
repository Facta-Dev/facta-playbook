#include "desktop/app/notes_window.h"
#include "desktop/infrastructure/storage/note_store.h"
#include "desktop/notes/adapters/sqlite_note_repository.h"
#include "desktop/notes/application/add_note.h"
#include "desktop/notes/application/list_notes.h"
#include "desktop/notes/presentation/notes_view_model.h"
#include "desktop/platform/app_paths.h"
#include "desktop/ui_kit/theme.h"

#include <QApplication>
#include <QString>
#include <QtGlobal>

namespace {

using AppWindow = desktop::app::NotesWindow;
using NotesViewModel = desktop::notes::presentation::NotesViewModel;

// The renderer must be chosen before the first window exists, so the flag is
// read from argv directly: QApplication::arguments() needs an instance.
bool hasFlag(int argc, char* argv[], const char* flag) {
    const QString needle = QString::fromLocal8Bit(flag);
    for (int i = 1; i < argc; ++i) {
        if (QString::fromLocal8Bit(argv[i]) == needle) {
            return true;
        }
    }
    return false;
}

}  // namespace

int main(int argc, char* argv[]) {
    const bool software = hasFlag(argc, argv, "--software-rendering");
    if (software) {
        // Compatibility mode (playbook 2.4): CPU rendering, no animations.
        qputenv("QT_QUICK_BACKEND", "software");
        qputenv("QT_OPENGL", "software");
    }

    QApplication app(argc, argv);
    QApplication::setOrganizationName(QStringLiteral("<scope>"));
    QApplication::setApplicationName(QStringLiteral("<app>"));
    QApplication::setApplicationVersion(QStringLiteral(DESKTOP_VERSION));
    app.setStyleSheet(desktop::ui_kit::styleSheet());

    if (software) {
        QApplication::setEffectEnabled(Qt::UI_AnimateCombo, false);
        QApplication::setEffectEnabled(Qt::UI_AnimateMenu, false);
        QApplication::setEffectEnabled(Qt::UI_AnimateTooltip, false);
    }

    // The composition root: the infrastructure, the repository, the use cases,
    // the view model and the window, wired together.
    const auto opened = desktop::infrastructure::storage::NoteStore::open(
        desktop::platform::AppPaths::databaseFile());
    if (!opened.ok()) {
        qCritical("cannot open the notes database: %s", qPrintable(opened.errorMessage()));
        return 1;
    }

    desktop::notes::adapters::SqliteNoteRepository repository(opened.value());
    desktop::notes::application::ListNotes listNotes(repository);
    desktop::notes::application::AddNote addNote(repository);
    NotesViewModel viewModel(listNotes, addNote);
    AppWindow window(&viewModel);

    QObject::connect(&viewModel, &NotesViewModel::notesLoaded, &window, &AppWindow::onNotesLoaded);
    QObject::connect(&viewModel, &NotesViewModel::errorRaised, &window,
                     &AppWindow::onErrorRaised);

    window.show();
    viewModel.refresh();

    return app.exec();
}
