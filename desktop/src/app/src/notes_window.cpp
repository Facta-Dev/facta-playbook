#include "desktop/app/notes_window.h"

#include "desktop/ui_kit/labeled_line_edit.h"
#include "desktop/ui_kit/list_panel.h"

#include <QHBoxLayout>
#include <QLabel>
#include <QPushButton>
#include <QVBoxLayout>
#include <QWidget>

namespace desktop::app {

NotesWindow::NotesWindow(notes::presentation::NotesViewModel* viewModel, QWidget* parent)
    : QMainWindow(parent), viewModel_(viewModel) {
    setWindowTitle(QStringLiteral("<app>"));
    buildUi();

    connect(addButton_, &QPushButton::clicked, this, [this]() {
        viewModel_->addNote(titleEdit_->text(), bodyEdit_->text());
    });
}

void NotesWindow::buildUi() {
    auto* central = new QWidget(this);
    central->setObjectName(QStringLiteral("notesWindow"));

    titleEdit_ = new ui_kit::LabeledLineEdit(QStringLiteral("Title"), central);
    bodyEdit_ = new ui_kit::LabeledLineEdit(QStringLiteral("Body"), central);
    bodyEdit_->setPlaceholderText(QStringLiteral("What happened?"));

    addButton_ = new QPushButton(QStringLiteral("Add note"), central);

    errorLabel_ = new QLabel(central);
    errorLabel_->setObjectName(QStringLiteral("uiKitErrorLabel"));
    errorLabel_->setWordWrap(true);
    errorLabel_->hide();

    emptyStateLabel_ = new QLabel(QStringLiteral("No notes yet."), central);
    emptyStateLabel_->setObjectName(QStringLiteral("uiKitEmptyState"));
    emptyStateLabel_->setAlignment(Qt::AlignCenter);
    emptyStateLabel_->hide();

    listPanel_ = new ui_kit::ListPanel(QStringLiteral("Notes"), central);
    listPanel_->setModel(viewModel_->notesModel());

    auto* form = new QHBoxLayout();
    form->addWidget(titleEdit_, 1);
    form->addWidget(bodyEdit_, 2);
    form->addWidget(addButton_, 0, Qt::AlignBottom);

    auto* layout = new QVBoxLayout(central);
    layout->addLayout(form);
    layout->addWidget(errorLabel_);
    layout->addWidget(listPanel_, 1);
    layout->addWidget(emptyStateLabel_);

    setCentralWidget(central);
    resize(960, 640);
}

void NotesWindow::onNotesLoaded() {
    emptyStateLabel_->setVisible(viewModel_->notesModel()->rowCount() == 0);
    errorLabel_->hide();
    titleEdit_->clear();
    bodyEdit_->clear();
}

void NotesWindow::onErrorRaised(const QString& message) {
    errorLabel_->setText(message);
    errorLabel_->show();
}

}  // namespace desktop::app
