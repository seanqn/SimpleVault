#include <QtTest/QtTest>
#include "vaultmanager.h"
#include "data/repository.h"

struct MockDB {
public:

};

class Test_VaultManager : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanupTestCase();
    void init();
    void cleanup();

    void test_startRowEdit(int index);
    void test_updateEditCache(const QString &role, const QString &value);
    void test_submitAndResetEditCache();
    void test_checkEditCacheEmpty();
    void test_checkEditCacheClean();

private:
    Repository* test_Repo;
    VaultManager* test_Manager;
    CredentialEditCache* test_Cache;
};

void TestController::initTestCase() {

}

void TestController::cleanupTestCase() {

}

void TestController::init() {
    test_Repo = new Repository(nullptr, ":memory:");
    test_Cache = new CredentialEditCache(nullptr);
    test_Manager = new VaultManager(test_Repo, test_Cache, nullptr);
}

void TestController::cleanup() {
    delete test_Manager;
    delete test_Repo;
    delete test_Cache;
}

void TestController::test_startRowEdit(int index) {

}

QTEST_MAIN(Test_VaultManager)
#include "tests_controller.moc"