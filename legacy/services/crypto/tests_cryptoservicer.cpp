#include <QObject>
#include <QtTest>
#include <QPair>
#include <QDebug>
#include "cryptotypes.h"
#include "cryptoservicer.h"

class TestCryptoServicer : public QObject {
    Q_OBJECT

private slots:
    void initTestCase();
    void cleanupTestCase();
    void init();
    void cleanup();

    // void encryptWithDerivedKey_data();
    // void encryptWithDerivedKey();

    void roundTripWithDerivedKey_data();
    void roundTripWithDerivedKey();

    // exceptions
    void encryptRejectInvalidKeySize();
    void encryptRejectInvalidPassword();

private:
    void deriveMockKey();

    CryptoServicer m_cryptService;
    const qsizetype keySize = static_cast<qsizetype>(crypto_aead_xchacha20poly1305_ietf_KEYBYTES);
    QByteArray m_mockKey = QByteArray(keySize, '\0');
};

// configure tests

void TestCryptoServicer::initTestCase() {
    QVERIFY2(sodium_init() >= 0, "libsodium init failed");

    deriveMockKey();
    QVERIFY2(!m_mockKey.isNull(), "mock key failed generation");
}

void TestCryptoServicer::cleanupTestCase() {

}

void TestCryptoServicer::init() {

}

void TestCryptoServicer::cleanup() {
    if (!m_mockKey.isNull()) m_mockKey.fill(0);
    if (!m_mockKey.isNull()) m_mockKey.fill(0);
}

void TestCryptoServicer::deriveMockKey() {
    crypto_aead_xchacha20poly1305_ietf_keygen(reinterpret_cast<unsigned char*>(m_mockKey.data()));
}

// test service

void TestCryptoServicer::roundTripWithDerivedKey_data() {
    QTest::addColumn<QString>("password");
    QTest::addColumn<bool>("shouldSucceed");

    // QTest::newRow("Empty") << "" << false;
    QTest::newRow("Weak1") << "WeakCamelCase" << true; // should pass in service but fail at input entrypoint
    // QTest::newRow("Weak2") << "6L}o" << false;
    QTest::newRow("Weak3") << "2U48nf" << true;
    QTest::newRow("Med1") << "hpLV72I9" << true;
    QTest::newRow("Med2") << "3,04C33k" << true;
    QTest::newRow("Med3") << "UZW8R7rAG9" << true;
    QTest::newRow("Strong1") << "iWy01t00wcSj" << true;
    QTest::newRow("Strong2") << "0EYvGQ9?44<d" << true;
    QTest::newRow("Strong3") << "A5ex7*^!7MH%" << true;
    QTest::newRow("Enterprise1") << "rKN16rQ81k[;<h{E" << true;
    QTest::newRow("Enterprise2") << "p3|yD5RFRPp:9{;5]" << true;
    // QTest::newRow("Enterprise3") << ";9O39_G\kJJ{Hc|KV~" << true; pw's with escape chars will have to be resolved
}

void TestCryptoServicer::roundTripWithDerivedKey() {
    QFETCH(QString, password);
    QFETCH(bool, shouldSucceed);

    auto encryptResult = m_cryptService.encrypt(password, reinterpret_cast<const unsigned char*>(m_mockKey.data()), m_mockKey.size());
    QCOMPARE(encryptResult.has_value(), shouldSucceed);

    if (!shouldSucceed) {
        QVERIFY(!encryptResult.has_value());
        return;
    }

    QVERIFY2(encryptResult.has_value(), "Encryption failed");

    auto decryptResult = m_cryptService.decrypt(encryptResult.value(), reinterpret_cast<const unsigned char*>(m_mockKey.data()), m_mockKey.size());
    QVERIFY2(decryptResult.has_value(), "Decryption failed");
    QCOMPARE(decryptResult.value(), password);

    qInfo().noquote() << "Decrypted:" << decryptResult.value() << "| CipherText Hex: " << encryptResult.value().secret.toHex();
}

// exceptions

void TestCryptoServicer::encryptRejectInvalidKeySize() {
    QByteArray invalidKey(16, 'x');
    auto result = m_cryptService.encrypt("F4%2Fa_3@4g", reinterpret_cast<const unsigned char*>(invalidKey.data()), invalidKey.size());

    QVERIFY(!result.has_value());
    QCOMPARE(result.error(), CryptoError::InvalidKeySize);
}

void TestCryptoServicer::encryptRejectInvalidPassword() {
    auto result = m_cryptService.encrypt("", reinterpret_cast<const unsigned char*>(m_mockKey.data()), m_mockKey.size());
    QVERIFY(!result.has_value());
    QCOMPARE(result.error(), CryptoError::InvalidInput);
}

QTEST_MAIN(TestCryptoServicer)
#include "tests_cryptoservicer.moc"
