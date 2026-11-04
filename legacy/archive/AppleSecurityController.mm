
// DEPRECATED: NEW BUILD USES QTKEYCHAIN
// CONTAINS OUTDATED IMPORTS
// FOR REFERENCE ONLY

#import "AppleSecurityController.h"
#import <Foundation/Foundation.h>
#import <AuthenticationServices/AuthenticationServices.h>
#import <Security/Security.h>
#import <AppKit/AppKit.h>
#import <QMetaObject>

#ifdef __cplusplus
class VaultSecurityService;
#endif

@interface AppleSecurityController : NSObject<ASAuthorizationControllerDelegate, ASAuthorizationControllerPresentationContextProviding>
- (void)unlockVault;
- (void)startPasskeyRegistration;
- (void)startPasskeyAuthentication;
- (void)generateStoreVaultKey;
- (void)performAuthRequest:(ASAuthorizationRequest *)request;
- (BOOL)vaultKeyExists;
- (NSData *)credentialID;
- (void)storeCredentialID:(NSData *)credentialID;
- (NSData *)loadStoredCredentialID;
- (NSData *)uuID;
- (void)storeUUID:(NSData *)uuID;
- (NSData *)loadUUID;
- (BOOL)storeKeychainData:(NSData *)data forAccount:(NSString *)account;

@property (nonatomic, strong) NSData *currentChallenge;
@property (nonatomic, strong) NSData *uuID;

@property (nonatomic, assign) VaultSecurityService *bridge;

@end

// vault states
typedef NS_ENUM(NSUInteger, VaultState) {
  VaultStateUnknown,
  VaultStateRequiresRegistration,
  VaultStateRequiresAuthentication,
  VaultStateUnlocked
};

@interface AppleSecurityController()
@property VaultState state;
@end

@implementation AppleSecurityController

- (void)unlockVault {
  if (![self vaultKeyExists]) {
    self.state = VaultStateRequiresRegistration;

    QMetaObject::invokeMethod(
      self.bridge,
      "authRequiresRegistration",
      Qt::QueuedConnection);

    [self startPasskeyRegistration];

  }
  else {
    self.state = VaultStateRequiresAuthentication;

    QMetaObject::invokeMethod(
      self.bridge,
      "authRequiresAuthentication",
      Qt::QueuedConnection);

    [self startPasskeyAuthentication];

  }
}

// verify vault key exists
- (BOOL)vaultKeyExists {
  NSDictionary *query = @{
    (__bridge id)kSecClass:
      (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrAccount:
      @"com.sean.simplevault.vaultkey",
    (__bridge id)kSecReturnData: @NO
  };

  OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, NULL);

  return status == errSecSuccess;
}

// auth challenge generation
- (NSData *)createChallenge {
  uint8_t bytes[32];
  SecRandomCopyBytes(kSecRandomDefault, sizeof(bytes), bytes);

  return [NSData dataWithBytes:bytes length:sizeof(bytes)];
}

// helper for updating keychain items
- (BOOL)storeKeychainData:(NSData *)data forAccount:(NSString *)account {
  // defining search for selected, repeatedly updated items
  NSDictionary *query = @ {
    (__bridge id)kSecClass:
      (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrAccount: account
  };

  // defines only data as selected attributes to update
  NSDictionary *attrsToUpdate = @{
    (__bridge id)kSecValueData: data
  };

  OSStatus status = SecItemUpdate((__bridge CFDictionaryRef) query, (__bridge CFDictionaryRef)attrsToUpdate);

  if (status == errSecItemNotFound) {
    NSMutableDictionary *attrsToAdd = [query mutableCopy];
    [attrsToAdd setObject:data forKey:(__bridge id)kSecValueData];

    [attrsToAdd setObject:(__bridge id)kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly forKey:(__bridge id)kSecAttrAccessible];

    status = SecItemAdd((__bridge CFDictionaryRef)attrsToAdd, NULL);
  }

  if (status != errSecSuccess) {
    NSLog(@"Keychain error for %@: %d", account, (int)status);
    return NO;
  }

  return YES;
}

// credential ID methods

// credential ID storage
- (void)storeCredentialID:(NSData *)credentialID {
  NSDictionary *attrs = @{
    (__bridge id)kSecClass:
      (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrAccount:
      @"com.sean.simplevault.passkey.credentialid",
    (__bridge id)kSecValueData:
      credentialID,
    (__bridge id)kSecAttrAccessible:
      (__bridge id)kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
  };

  [self storeKeychainData:credentialID forAccount:@"com.sean.simplevault.passkey.credentialid"];
}

// credential ID fetching
- (NSData *)loadStoredCredentialID {
  NSDictionary *query = @{
    (__bridge id)kSecClass:
      (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrAccount:
      @"com.sean.simplevault.passkey.credentialid",
    (__bridge id)kSecReturnData: @YES
  };

  CFTypeRef result = NULL;
  OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &result);

  if (status != errSecSuccess) {
    return nil;
  }

  return (__bridge_transfer NSData *)result;
}

// uuid methods

// uuid generation
- (NSData *)uuID {
  if (_uuID) return _uuID;

  NSData *existingID = [self loadUUID];
  if (existingID) {
    _uuID = existingID;
    return existingID;
  }

  uint8_t bytes[16];
  SecRandomCopyBytes(kSecRandomDefault, sizeof(bytes), bytes);
  NSData *newUUID = [NSData dataWithBytes:bytes length:sizeof(bytes)];

  [self storeUUID:newUUID];
  _uuID = newUUID;

  return _uuID;
}

// uuid storage
- (void)storeUUID:(NSData *)uuID {
  NSDictionary *attrs = @{
    (__bridge id)kSecClass:
      (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrAccount:
      @"com.sean.simplevault.vault.UUID",
    (__bridge id)kSecValueData:
      uuID,
    (__bridge id)kSecAttrAccessible:
      (__bridge id)kSecAttrAccessibleAfterFirstUnlockThisDeviceOnly
  };

  [self storeKeychainData:uuID forAccount:@"com.sean.simplevault.vault.UUID"];
}

// uuid fetching
- (NSData *)loadUUID {
  NSDictionary *query = @{
    (__bridge id)kSecClass:
      (__bridge id)kSecClassGenericPassword,
    (__bridge id)kSecAttrAccount:
      @"com.sean.simplevault.vault.UUID",
    (__bridge id)kSecReturnData: @YES
  };

  CFTypeRef result = NULL;
  OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)query, &result);

  if (status != errSecSuccess) {
    return nil;
  }

  return (__bridge_transfer NSData *)result;
}

// vault key generation
- (void)generateStoreVaultKey {
  uint8_t keyBytes[32];
  SecRandomCopyBytes(kSecRandomDefault, sizeof(keyBytes), keyBytes);

  NSData *keyData = [NSData dataWithBytes:keyBytes length:sizeof(keyBytes)];

  [self storeKeychainData:keyData forAccount:@"com.sean.simplevault.vaultkey"];

  memset_s(keyBytes, sizeof(keyBytes), 0, sizeof(keyBytes));
}

// passkey registration/authentication methods
- (void)startPasskeyRegistration {
  self.currentChallenge = [self createChallenge];
  ASAuthorizationPlatformPublicKeyCredentialProvider *provider = [[ASAuthorizationPlatformPublicKeyCredentialProvider alloc]initWithRelyingPartyIdentifier:@"com.sean.simplevault"];

  ASAuthorizationPlatformPublicKeyCredentialRegistrationRequest *request = [
    provider createCredentialRegistrationRequestWithChallenge:self.currentChallenge
      name:@"Vault Ownder"
      userID:[self uuID]
  ];

  [self performAuthRequest:request];
}

- (void)startPasskeyAuthentication {
  self.currentChallenge = [self createChallenge];
  ASAuthorizationPlatformPublicKeyCredentialProvider *provider = [[ASAuthorizationPlatformPublicKeyCredentialProvider alloc]initWithRelyingPartyIdentifier:@"com.sean.simplevault"];

  ASAuthorizationPlatformPublicKeyCredentialAssertionRequest *request = [
    provider createCredentialAssertionRequestWithChallenge:self.currentChallenge
  ];

  [self performAuthRequest:request];
}

- (void)performAuthRequest:(ASAuthorizationRequest *)request {
  ASAuthorizationController *controller = [[ASAuthorizationController alloc]initWithAuthorizationRequests:@[request]];

  controller.delegate = self;
  controller.presentationContextProvider = self;
  [controller performRequests];
}

// auth controller/signals
- (void)authorizationController:(ASAuthorizationController *)controller
didCompleteWithAuthorization:(ASAuthorization *)authorization {
  id credential = authorization.credential;

  if (self.state == VaultStateRequiresRegistration) {

    ASAuthorizationPlatformPublicKeyCredentialRegistration *registration = (ASAuthorizationPlatformPublicKeyCredentialRegistration *)credential;

    if (![credential isKindOfClass: ASAuthorizationPlatformPublicKeyCredentialRegistration.class]) return;

    NSData *credentialID = registration.credentialID;

    [self storeCredentialID:credentialID];
    [self generateStoreVaultKey];
    self.state = VaultStateUnlocked;

    QMetaObject::invokeMethod(
      self.bridge,
      "vaultUnlocked",
      Qt::QueuedConnection);

    return;
  }

  if (self.state == VaultStateRequiresAuthentication) {

    ASAuthorizationPlatformPublicKeyCredentialAssertion *assertion = (ASAuthorizationPlatformPublicKeyCredentialAssertion *)credential;

    // NSData* newcredentialID = assertion.credentialID;

    if (![credential isKindOfClass: ASAuthorizationPlatformPublicKeyCredentialAssertion.class]) return;

    NSData* storedCredentialID = [self loadStoredCredentialID];

    if (![assertion.credentialID isEqualToData:storedCredentialID]) {
      self.state = VaultStateUnknown;

    QMetaObject::invokeMethod(
      self.bridge,
      "authFailed",
      Qt::QueuedConnection);

      return;
    }

    if (self.state == VaultStateUnlocked) {
      if (self.bridge) {
        QMetaObject::invokeMethod(
          self.bridge,
          "vaultUnlocked",
          Qt::QueuedConnection);
      }
    }

    return;
  }
}

- (void)authorizationController:(ASAuthorizationController *)controller
didCompleteWithError:(NSError *)error {
    self.state = VaultStateUnknown;

    NSString *objcError = error.localizedDescription;

    QString qError = QString::fromNSString(objcError);

    if (self.bridge) {
      QMetaObject::invokeMethod(
        self.bridge,
        "authFailed",
        Qt::QueuedConnection,
        Q_ARG(QString, qError));
    }
}

// preprocessor platform dependent UI construction
- (ASPresentationAnchor)presentationAnchorForAuthorizationController:
(ASAuthorizationController *)controller {
  #if TARGET_OS_IPHONE
    for (UIScene *scene in UIApplication.sharedApplication.connectedScenes) {
      if ([scene isKindOfClass:[UIWindowScene class]] && scene.activationState == UISceneActivationStateForegroundActive) {
        UIWindowScene *windowScene = (UIWindowScene *)scene;
        return windowScene.windows.firstObject;
      }
    }
    return nil;

  #else
    return NSApplication.sharedApplication.keyWindow
      ?: NSApplication.sharedApplication.mainWindow;

  #endif
}

@end

// C++ wrapper
VaultSecurityService::VaultSecurityService(QObject *parent) : QObject(parent) {
  AppleSecurityController *controller = [[AppleSecurityController alloc] init];
  controller.bridge = this;

  objcController = (__bridge_retained void*)controller;

  setStatusText("Idle");
}

// frees for both C++ and objC objects
VaultSecurityService::~VaultSecurityService() {
  if (objcController) {
    AppleSecurityController *controller = (__bridge AppleSecurityController*)objcController;

    controller.bridge = nullptr;

    CFRelease(objcController);
    objcController = nullptr;
  }
}

void VaultSecurityService::unlockVault() {
  [(__bridge AppleSecurityController*)objcController unlockVault];

  setBusy(true);
  setStatusText("Prompting device passkey...");
}

// slots
void VaultSecurityService::vaultUnlocked() {
  setBusy(false);
  setStatusText("Vault unlocked");
  emit unlockSuccessful();
}

void VaultSecurityService::authRequiresRegistration() {
  setBusy(true);
  setStatusText("Registration in process...");
}

void VaultSecurityService::authRequiresAuthentication() {
  setBusy(true);
  setStatusText("Authentication in process...");
}

void VaultSecurityService::authFailed(QString error) {
  setBusy(false);
  setStatusText("Authentication Failed: " + error);
  emit unlockFailed();
}


// signal helpers
QString VaultSecurityService::statusText() const {
  return m_statusText;
}

bool VaultSecurityService::busy() const {
  return m_busy;
}

void VaultSecurityService::setStatusText(const QString &text) {
  if (m_statusText == text) {
    return;
  }

  m_statusText = text;
  emit statusTextChanged();
}

void VaultSecurityService::setBusy(bool value) {
  if (m_busy == value) {
    return;
  }

  m_busy = value;
  emit busyChanged();
}
