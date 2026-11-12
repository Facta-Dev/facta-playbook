#pragma once

#include <QString>
#include <QWidget>

class QAbstractItemModel;
class QLabel;
class QListView;

namespace desktop::ui_kit {

// A titled panel around a list view.
class ListPanel : public QWidget {
    Q_OBJECT

public:
    explicit ListPanel(const QString& title, QWidget* parent = nullptr);

    // The model of the list.
    void setModel(QAbstractItemModel* model);

    [[nodiscard]] QListView* listView() const noexcept { return view_; }

private:
    QLabel* title_;
    QListView* view_;
};

}  // namespace desktop::ui_kit
