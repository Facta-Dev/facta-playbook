#pragma once

#include <QString>
#include <QWidget>

class QLabel;
class QLineEdit;

namespace desktop::ui_kit {

// A text field with its label: the form of every feature uses it.
class LabeledLineEdit : public QWidget {
    Q_OBJECT

public:
    explicit LabeledLineEdit(const QString& label, QWidget* parent = nullptr);

    [[nodiscard]] QString text() const;
    void setText(const QString& text);
    void setPlaceholderText(const QString& placeholder);
    void clear();

private:
    QLabel* label_;
    QLineEdit* edit_;
};

}  // namespace desktop::ui_kit
