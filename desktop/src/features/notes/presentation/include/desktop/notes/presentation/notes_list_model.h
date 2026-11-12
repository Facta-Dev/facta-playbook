#pragma once

#include "desktop/notes/domain/note.h"

#include <QAbstractListModel>
#include <QVector>

namespace desktop::notes::presentation {

// The list of notes behind the view: a plain item model, so the view never
// touches the use cases.
class NotesListModel : public QAbstractListModel {
    Q_OBJECT

public:
    enum Roles {
        IdRole = Qt::UserRole + 1,
        TitleRole,
        BodyRole,
        CreatedAtRole,
        ToolTipRole
    };

    explicit NotesListModel(QObject* parent = nullptr);

    // Replaces the whole list.
    void setNotes(const QVector<domain::Note>& notes);

    [[nodiscard]] int rowCount(const QModelIndex& parent = QModelIndex()) const override;
    [[nodiscard]] QVariant data(const QModelIndex& index, int role) const override;
    [[nodiscard]] QHash<int, QByteArray> roleNames() const override;

private:
    QVector<domain::Note> notes_;
};

}  // namespace desktop::notes::presentation
