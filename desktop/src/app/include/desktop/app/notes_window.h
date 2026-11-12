#pragma once

#include "desktop/notes/presentation/notes_view_model.h"

#include <QMainWindow>
#include <QString>

class QLabel;
class QPushButton;

namespace desktop::ui_kit {
class LabeledLineEdit;
class ListPanel;
}  // namespace desktop::ui_kit

namespace desktop::app {

// The window of the notes: the form, the list and the messages. It knows the
// view model and the UI kit, never the use cases and never the storage.
class NotesWindow : public QMainWindow {
    Q_OBJECT

public:
    explicit NotesWindow(notes::presentation::NotesViewModel* viewModel, QWidget* parent = nullptr);

public slots:
    // The composition root connects the view model to the view.
    void onNotesLoaded();
    void onErrorRaised(const QString& message);

private:
    void buildUi();

    notes::presentation::NotesViewModel* viewModel_;
    ui_kit::LabeledLineEdit* titleEdit_;
    ui_kit::LabeledLineEdit* bodyEdit_;
    ui_kit::ListPanel* listPanel_;
    QPushButton* addButton_;
    QLabel* emptyStateLabel_;
    QLabel* errorLabel_;
};

}  // namespace desktop::app
