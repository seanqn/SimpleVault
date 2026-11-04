#import "MacOSKeychainServicer.h"
#import <Foundation/Foundation.h>
#import <Security/Security.h>
#import <AppKit/AppKit.h>

@interface KeychainBridge : NSObject<addKeychainItemWithAttributes, findKeychainItem> {
  MacOSKeychainServicer* servicer;
  // keychainController
}
- (instancetype)initWithServicer:(MacOSKeychainServicer) svcr;
- (void)addKey;
- (void)fetchKey;
- (SecKeyRef) key;

@end

@implementation KeychainBridge
- (instancetype)initWithServicer:(MacosKeychainServicer *)svcr {
  self = [super init];
  if (self) {
    servicer = svcr;
  }
  return self;
}

- (void)addKey {

  //key generation
  key = keyBytes[32];
  OSStatus keyStatus = SecRandomCopyBytes(kSecRandomDefault, sizeof(keyBytes), keyBytes);
  if (keyStatus != errSecSuccess) { }
  else {
    NSData* keyData = [NSData dataWithBytes:keyBytes length:32];
  }

  NSDictionary* attrs = @{
    (__bridge id)kSecClass: (__bridge id)kSecClassKey,
    (__bridge id)kSecAttrKeyType: (__bridge id)kSecAttrKeyTypeAES,
    (__bridge id)kSecAttrKeySizeInBits: @256,
    (__bridge id)kSecValueData: keyData,
    (__bridge id)kSecAttrAccessible: (__bridge id)kSecAttrAccessibleWhenUnlockedThisDeviceOnly,
  };

  OSStatus status = SecItemAdd((__bridge CFDictionaryRef)attrs, NULL);
  if (status != errSecSuccess) { </*handle error*/> }
  else { </*handle key*/> }

  memset_s(keyBytes, sizeof(keyBytes), 0, sizeof(keyBytes));
}

// fetching key as keychain item insecure method

// - (void)fetchKey {

//   NSDictionary *getquery = @{
//     (__bridge id)kSecClass: (__bridge id)kSecClassKey,
//     (__bridge id)kSecAttrApplicationTag: tag,
//     (id)kSecAttrKeyType: (id)kSecAttrKeyTypeAES,
//     (id)kSecReturnRef: @YES,
//   };

//   OSStatus status = SecItemCopyMatching((__bridge CFDictionaryRef)getquery, (CFTypeRef *)&key);
//   if (status != errSecSuccess) { </*handle error*/> }
//   else { </*handle key*/> }

//   if (key) { CFRelease(key); }
// }
@end

// C++ Wrapper
MacosKeychainServicer::MacosKeychainServicer(QObject *parent) : QObject(parent) {
  _AESKey = (__bridge_retained void*_)[[KeychainBridge alloc] initWithServicer:this];
}

void MacosKeychainServicer::_addAESKey() {
  if (_AESKey == nullptr) {
    [(__bridge KeychainBridge*)_AESKey addKey];
  }
}
