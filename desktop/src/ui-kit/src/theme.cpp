#include "desktop/ui_kit/theme.h"

namespace desktop::ui_kit {

QString styleSheet() {
    return QStringLiteral(
        "QWidget#notesWindow { background: #f6f7f9; }\n"
        "QLabel { color: #1f2430; }\n"
        "QLabel#uiKitFieldLabel { color: #5a6472; }\n"
        "QLabel#uiKitPanelTitle { color: #1f2430; font-weight: 600; }\n"
        "QLabel#uiKitEmptyState { color: #7a8494; }\n"
        "QLabel#uiKitErrorLabel { color: #b3261e; }\n"
        "QLineEdit { padding: 6px 8px; border: 1px solid #c8ccd4; border-radius: 4px; "
        "background: #ffffff; }\n"
        "QLineEdit:focus { border: 1px solid #2f6feb; }\n"
        "QPushButton { padding: 6px 14px; border: 1px solid #c8ccd4; border-radius: 4px; "
        "background: #ffffff; }\n"
        "QPushButton:hover { background: #eef2fb; }\n"
        "QPushButton:pressed { background: #dfe7fb; }\n"
        "QListView { border: 1px solid #dfe3ea; border-radius: 4px; background: #ffffff; }\n");
}

}  // namespace desktop::ui_kit
