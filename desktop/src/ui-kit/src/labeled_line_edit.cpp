#include "desktop/ui_kit/labeled_line_edit.h"

#include <QLabel>
#include <QLineEdit>
#include <QVBoxLayout>

namespace desktop::ui_kit {

LabeledLineEdit::LabeledLineEdit(const QString& label, QWidget* parent)
    : QWidget(parent), label_(new QLabel(label, this)), edit_(new QLineEdit(this)) {
    label_->setObjectName(QStringLiteral("uiKitFieldLabel"));

    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);
    layout->addWidget(label_);
    layout->addWidget(edit_);
}

QString LabeledLineEdit::text() const {
    return edit_->text();
}

void LabeledLineEdit::setText(const QString& text) {
    edit_->setText(text);
}

void LabeledLineEdit::setPlaceholderText(const QString& placeholder) {
    edit_->setPlaceholderText(placeholder);
}

void LabeledLineEdit::clear() {
    edit_->clear();
}

}  // namespace desktop::ui_kit
