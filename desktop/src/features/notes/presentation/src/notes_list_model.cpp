#include "desktop/notes/presentation/notes_list_model.h"

#include <QVariant>

namespace desktop::notes::presentation {

NotesListModel::NotesListModel(QObject* parent) : QAbstractListModel(parent) {}

void NotesListModel::setNotes(const QVector<domain::Note>& notes) {
    beginResetModel();
    notes_ = notes;
    endResetModel();
}

int NotesListModel::rowCount(const QModelIndex& parent) const {
    return parent.isValid() ? 0 : notes_.size();
}

QVariant NotesListModel::data(const QModelIndex& index, int role) const {
    if (!index.isValid() || index.row() < 0 || index.row() >= notes_.size()) {
        return QVariant();
    }

    const domain::Note& note = notes_.at(index.row());
    switch (role) {
    case Qt::DisplayRole:
    case TitleRole:
        return note.title();
    case IdRole:
        return note.id().toString();
    case BodyRole:
        return note.body();
    case ToolTipRole:
        return note.body();
    case CreatedAtRole:
        return note.createdAt().toIso8601();
    default:
        return QVariant();
    }
}

QHash<int, QByteArray> NotesListModel::roleNames() const {
    return {
        {IdRole, QByteArrayLiteral("noteId")},
        {TitleRole, QByteArrayLiteral("title")},
        {BodyRole, QByteArrayLiteral("body")},
        {CreatedAtRole, QByteArrayLiteral("createdAt")},
    };
}

}  // namespace desktop::notes::presentation
