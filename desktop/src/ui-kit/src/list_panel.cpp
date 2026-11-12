#include "desktop/ui_kit/list_panel.h"

#include <QAbstractItemModel>
#include <QLabel>
#include <QListView>
#include <QVBoxLayout>

namespace desktop::ui_kit {

ListPanel::ListPanel(const QString& title, QWidget* parent)
    : QWidget(parent), title_(new QLabel(title, this)), view_(new QListView(this)) {
    title_->setObjectName(QStringLiteral("uiKitPanelTitle"));

    auto* layout = new QVBoxLayout(this);
    layout->setContentsMargins(0, 0, 0, 0);
    layout->setSpacing(4);
    layout->addWidget(title_);
    layout->addWidget(view_, 1);
}

void ListPanel::setModel(QAbstractItemModel* model) {
    view_->setModel(model);
}

}  // namespace desktop::ui_kit
