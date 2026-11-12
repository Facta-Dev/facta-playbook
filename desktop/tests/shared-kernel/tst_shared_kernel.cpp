// The shared kernel: the value types every layer depends on.

#include "desktop/kernel/ids.h"
#include "desktop/kernel/result.h"

#include <QTest>

namespace {

using desktop::kernel::ErrorCode;
using desktop::kernel::NoteId;
using desktop::kernel::Result;

}  // namespace

class SharedKernelTests : public QObject {
    Q_OBJECT

private slots:
    void resultCarriesAValue();
    void resultCarriesAnError();
    void resultFallsBackToTheGivenValue();
    void resultWithoutAValueCarriesOnlyTheError();
    void strongIdKeepsItsValue();
    void strongIdIsInvalidWhenEmpty();
    void strongIdComparesByValue();
};

void SharedKernelTests::resultCarriesAValue() {
    const Result<int> ok(7);

    QVERIFY(ok.ok());
    QCOMPARE(ok.value(), 7);
    QCOMPARE(ok.errorCode(), ErrorCode::Unknown);
    QVERIFY(ok.errorMessage().isEmpty());
}

void SharedKernelTests::resultCarriesAnError() {
    const Result<int> failed(ErrorCode::Storage, QStringLiteral("disk full"));

    QVERIFY(!failed.ok());
    QCOMPARE(failed.errorCode(), ErrorCode::Storage);
    QCOMPARE(failed.errorMessage(), QStringLiteral("disk full"));
}

void SharedKernelTests::resultFallsBackToTheGivenValue() {
    const Result<int> failed(ErrorCode::Storage, QStringLiteral("disk full"));

    QCOMPARE(failed.valueOr(3), 3);
    QCOMPARE(Result<int>(9).valueOr(3), 9);
}

void SharedKernelTests::resultWithoutAValueCarriesOnlyTheError() {
    const Result<void> empty;

    QVERIFY(empty.ok());
    QCOMPARE(empty.errorCode(), ErrorCode::Unknown);
    QVERIFY(empty.errorMessage().isEmpty());

    const Result<void> failed(ErrorCode::NotFound, QStringLiteral("gone"));

    QVERIFY(!failed.ok());
    QCOMPARE(failed.errorCode(), ErrorCode::NotFound);
    QCOMPARE(failed.errorMessage(), QStringLiteral("gone"));
}

void SharedKernelTests::strongIdKeepsItsValue() {
    const NoteId id(QStringLiteral("n-1"));

    QCOMPARE(id.value(), QStringLiteral("n-1"));
    QCOMPARE(id.toString(), QStringLiteral("n-1"));
    QVERIFY(id.isValid());
}

void SharedKernelTests::strongIdIsInvalidWhenEmpty() {
    const NoteId id;

    QVERIFY(!id.isValid());
    QVERIFY(id.value().isEmpty());
}

void SharedKernelTests::strongIdComparesByValue() {
    const NoteId first(QStringLiteral("n-1"));
    const NoteId same(QStringLiteral("n-1"));
    const NoteId other(QStringLiteral("n-2"));

    QCOMPARE(first, same);
    QVERIFY(first != other);
}

QTEST_MAIN(SharedKernelTests)
#include "tst_shared_kernel.moc"
