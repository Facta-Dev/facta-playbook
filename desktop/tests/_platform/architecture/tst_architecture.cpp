// The architecture guard: the layering rules of the playbook, checked on the
// source tree itself.
//
//   1. the domain of a slice never includes a QtWidgets header;
//   2. an OS-specific header is only included inside src/platform;
//   3. the application layer of a slice never includes an infrastructure
//      header.
//
// The rules are read from the sources, not from a hand-written list of the
// files that exist today, so a new file is checked as well.

#include <QDir>
#include <QDirIterator>
#include <QFile>
#include <QFileInfo>
#include <QLatin1String>
#include <QRegularExpression>
#include <QString>
#include <QStringList>
#include <QTest>

namespace {

// The root of the source tree, given by the build.
const QString& sourceRoot() {
    static const QString kRoot = QStringLiteral(DESKTOP_SOURCE_ROOT);
    return kRoot;
}

// The include directory of the QtWidgets module, given by the build.
const QString& qtWidgetsIncludeDir() {
    static const QString kDir = QStringLiteral(DESKTOP_QT_WIDGETS_INCLUDE_DIR);
    return kDir;
}

// Every source file of the tree.
QStringList sourceFiles() {
    QStringList files;
    const QStringList roots = {QStringLiteral("src"), QStringLiteral("packages")};
    for (const QString& root : roots) {
        QDirIterator it(sourceRoot() + QLatin1Char('/') + root, QDir::Files,
                        QDirIterator::Subdirectories);
        while (it.hasNext()) {
            const QString path = it.next();
            const QString suffix = QFileInfo(path).suffix();
            if (suffix == QStringLiteral("cpp") || suffix == QStringLiteral("h")) {
                files.append(path);
            }
        }
    }
    files.sort();
    return files;
}

// The headers a file includes.
QStringList includesOf(const QString& path) {
    QFile file(path);
    if (!file.open(QIODevice::ReadOnly | QIODevice::Text)) {
        return {};
    }
    const QString content = QString::fromUtf8(file.readAll());

    static const QRegularExpression kInclude(QStringLiteral("#\\s*include\\s*[<\"]([^>\"]+)[>\"]"));
    QStringList includes;
    QRegularExpressionMatchIterator matches = kInclude.globalMatch(content);
    while (matches.hasNext()) {
        includes.append(matches.next().captured(1));
    }
    return includes;
}

// A header of the QtWidgets module: a file that lives in the include
// directory of the module.
bool isQtWidgetsHeader(const QString& include) {
    const QString name = include.section(QLatin1Char('/'), -1);
    if (name.isEmpty()) {
        return false;
    }
    return QFileInfo::exists(qtWidgetsIncludeDir() + QLatin1Char('/') + name);
}

// The headers only an operating system provides.
const QStringList& osSpecificHeaders() {
    static const QStringList kHeaders = {
        QStringLiteral("windows.h"),    QStringLiteral("winapi.h"),
        QStringLiteral("winuser.h"),    QStringLiteral("objbase.h"),
        QStringLiteral("ole2.h"),       QStringLiteral("oleauto.h"),
        QStringLiteral("shlwapi.h"),    QStringLiteral("comctl32.h"),
        QStringLiteral("winsock2.h"),   QStringLiteral("ws2tcpip.h"),
        QStringLiteral("iphlpapi.h"),   QStringLiteral("process.h"),
        QStringLiteral("direct.h"),     QStringLiteral("io.h"),
        QStringLiteral("fcntl.h"),      QStringLiteral("unistd.h"),
        QStringLiteral("dirent.h"),     QStringLiteral("dlfcn.h"),
        QStringLiteral("pthread.h"),    QStringLiteral("sched.h"),
        QStringLiteral("termios.h"),    QStringLiteral("pwd.h"),
        QStringLiteral("grp.h"),        QStringLiteral("syslog.h"),
        QStringLiteral("netdb.h"),      QStringLiteral("arpa/inet.h"),
        QStringLiteral("netinet/in.h"), QStringLiteral("sys/socket.h"),
        QStringLiteral("sys/types.h"),  QStringLiteral("sys/stat.h"),
        QStringLiteral("sys/mman.h"),   QStringLiteral("sys/wait.h"),
        QStringLiteral("sys/time.h"),   QStringLiteral("sys/ioctl.h"),
        QStringLiteral("sys/un.h"),     QStringLiteral("sys/utsname.h"),
        QStringLiteral("mach/mach.h"),  QStringLiteral("objc/objc.h"),
        QStringLiteral("CoreFoundation/CoreFoundation.h"),
        QStringLiteral("Foundation/Foundation.h"),
        QStringLiteral("AppKit/AppKit.h"),
        QStringLiteral("IOKit/IOKitLib.h"),
        QStringLiteral("Carbon/Carbon.h"),
        QStringLiteral("X11/Xlib.h"),   QStringLiteral("X11/Xutil.h")};
    return kHeaders;
}

// The directories that hold only operating system headers.
const QStringList& osSpecificPrefixes() {
    static const QStringList kPrefixes = {
        QStringLiteral("sys/"),         QStringLiteral("mach/"),
        QStringLiteral("objc/"),        QStringLiteral("arpa/"),
        QStringLiteral("netinet/"),     QStringLiteral("CoreFoundation/"),
        QStringLiteral("Foundation/"),  QStringLiteral("AppKit/"),
        QStringLiteral("IOKit/"),       QStringLiteral("Carbon/"),
        QStringLiteral("X11/")};
    return kPrefixes;
}

bool isOsSpecificHeader(const QString& include) {
    if (osSpecificHeaders().contains(include)) {
        return true;
    }
    for (const QString& prefix : osSpecificPrefixes()) {
        if (include.startsWith(prefix)) {
            return true;
        }
    }
    return false;
}

// The relative path of a file, with forward slashes.
QString relativePath(const QString& path) {
    return QDir::fromNativeSeparators(path).mid(sourceRoot().size() + 1);
}

bool isDomainFile(const QString& relative) {
    return relative.startsWith(QStringLiteral("src/features/")) &&
           relative.contains(QStringLiteral("/domain/"));
}

bool isApplicationFile(const QString& relative) {
    return relative.startsWith(QStringLiteral("src/features/")) &&
           relative.contains(QStringLiteral("/application/"));
}

bool isPlatformFile(const QString& relative) {
    return relative.startsWith(QStringLiteral("src/platform/"));
}

// The violations of one rule, as readable lines.
QStringList violations(bool (*rule)(const QString& relative, const QString& include),
                        const QStringList& files) {
    QStringList found;
    for (const QString& path : files) {
        const QString relative = relativePath(path);
        for (const QString& include : includesOf(path)) {
            if (rule(relative, include)) {
                found.append(relative + QStringLiteral(" includes ") + include);
            }
        }
    }
    return found;
}

bool domainHasNoWidgetInclude(const QString& relative, const QString& include) {
    return isDomainFile(relative) && isQtWidgetsHeader(include);
}

bool onlyPlatformHasOsInclude(const QString& relative, const QString& include) {
    return !isPlatformFile(relative) && isOsSpecificHeader(include);
}

bool applicationHasNoInfrastructureInclude(const QString& relative, const QString& include) {
    return isApplicationFile(relative) && include.contains(QStringLiteral("infrastructure"));
}

}  // namespace

class ArchitectureTests : public QObject {
    Q_OBJECT

private slots:
    void findsTheSourceTree();
    void theDomainIncludesNoWidgetHeader();
    void onlyThePlatformIncludesOsSpecificHeaders();
    void theApplicationIncludesNoInfrastructureHeader();
};

void ArchitectureTests::findsTheSourceTree() {
    QVERIFY2(QFileInfo(sourceRoot()).isDir(), qPrintable(sourceRoot()));
    QVERIFY(!qtWidgetsIncludeDir().isEmpty());
    QVERIFY2(QFileInfo(qtWidgetsIncludeDir()).isDir(), qPrintable(qtWidgetsIncludeDir()));
    // The detection of a widget header only works when the module is found.
    QVERIFY(QFileInfo::exists(qtWidgetsIncludeDir() + QStringLiteral("/QWidget")));
    QVERIFY(sourceFiles().size() > 10);
}

void ArchitectureTests::theDomainIncludesNoWidgetHeader() {
    const QStringList found = violations(domainHasNoWidgetInclude, sourceFiles());

    QVERIFY2(found.isEmpty(), qPrintable(found.join(QStringLiteral("\n"))));
}

void ArchitectureTests::onlyThePlatformIncludesOsSpecificHeaders() {
    const QStringList found = violations(onlyPlatformHasOsInclude, sourceFiles());

    QVERIFY2(found.isEmpty(), qPrintable(found.join(QStringLiteral("\n"))));
}

void ArchitectureTests::theApplicationIncludesNoInfrastructureHeader() {
    const QStringList found = violations(applicationHasNoInfrastructureInclude, sourceFiles());

    QVERIFY2(found.isEmpty(), qPrintable(found.join(QStringLiteral("\n"))));
}

QTEST_MAIN(ArchitectureTests)
#include "tst_architecture.moc"
