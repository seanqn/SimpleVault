// FILE UNFINISHED AND NEEDS SEVERAL CHECKS
// SDK IMPLEMENTATIONS REQUIRE CHECKS

// delegates allow callbacks such as success or failure to notify
#import "MacosPasskeyServicer.h"
#import <Foundation/Foundation.h>
#import <AuthenticationServices/AuthenticationServices.h>
#import <Appkit/AppKit.h>

// OBJC BEGIN

// Declare the actual ObjC class that wraps C++ objects
@interface PasskeyBridge : NSObject <ASAuthorizationControllerDelegate, ASAuthorizationControllerPresentationContextProviding> {
  MacosPasskeyServicer* servicer;
  ASAuthorizationController* _controller;
}
- (instancetype)initWithServicer:(MacosPasskeyServicer *)svcr;
- (void)startRegistration;
- (void)startAuthentication;
@end

@implementation PasskeyBridge
- (instancetype)initWithServicer:(MacosPasskeyServicer *)svcr {
  self = [super init];
  if (self) {
    servicer = svcr;
  }
  return self;
}

- (void)startRegistration {
  ASAuthorizationPlatformPublicKeyCredentialProvider *provider = [
    [ASAuthorizationPlatformPublicKeyCredentialProvider alloc] initWithRelyingPartyIdentifier:
    @"SimpleVault"
  ];

  // passkey registration request
  ASAuthorizationPlatformPublicKeyCredentialRegistrationRequest *request = [
    provider createCredentialRegistrationRequestWithChallenge:[
      @"randomChallengeFromServer" dataUsingEncoding:NSUTF8StringEncoding]
      name: @"UserAccountName"
      userID:[@"uniqueUserID" dataUsingEncoding:NSUTF8StringEncoding]
  ];

  // controller
  _controller = [
    [ASAuthorizationController alloc] initWithAuthorizationRequests:@[request]
  ];

  _controller.delegate = self;
  _controller.presentationContextProvider = self;

  [_controller performRequests];
}

- (void)startAuthentication {
  ASAuthorizationPlatformPublicKeyCredentialProvider *provider = [
    [ASAuthorizationPlatformPublicKeyCredentialProvider alloc] initWithRelyingPartyIdentifier:
    @"SimpleVault"
  ];

  // passkey sign-in request
  ASAuthorizationPlatformPublicKeyCredentialAssertionRequest *request = [
    provider createCredentialAssertionRequestWithChallenge:
      [@"randomChallengeFromServer" dataUsingEncoding:NSUTF8StringEncoding]
  ];

  _controller = [
    [ASAuthorizationController alloc] initWithAuthorizationRequests:@[request]
  ];

  _controller.delegate = self;
  _controller.presentationContextProvider = self;

  [_controller performRequests];
}

- (ASPresentationAnchor)presentationAnchorForAuthorizationController:(ASAuthorizationController *)controller {
  // Notify macOS to open authentication method dialog over given window
  return [NSApp.windows firstObject];
}

// ASAuthorizationController manages authorization requests that a provider creates

// Callbacks
- (void)authorizationControllerSuccess:(ASAuthorizationController *)controllerdidCompleteWithAuthorization:(ASAuthorization *)authorization {

  // For successful registration
  if ([authorization.credential conformsToProtocol:@protocol(ASAuthorizationPublicKeyCredentialRegistration)]) {

    // Bridge callback data to C++
    emit servicer->authSuccessful(QString::fromNSString(@"Passkey Registration Successful"));
  }
  // For successful sign-in
  else if ([authorization.credential conformsToProtocol:@protocol(ASAuthorizationPublicKeyCredentialAssertion)]) {

    emit servicer->authSuccessful(QString::fromNSString(@"Passkey Authentication Successful"));
  }
}

// Failure callback
- (void)authorizationControllerFailure:(ASAuthorizationController *)controllerdidCompleteWithError:(NSError *)error {

  emit servicer->authFailed(QString::fromNSString([error localizedDescription]));
}
@end

// OBJC END


// C++ Wrapper

// MacosPasskeyServicer implement ObjC objects to become usable in project
MacosPasskeyServicer::MacosPasskeyServicer(QObject *parent) : QObject(parent) {

  // __bridge_retained prevents ptr from getting automatically released by ARC compilers
  // determine valid functionality
  objcBridge = (__bridge_retained void*)[[PasskeyBridge alloc] initWithServicer:this];
}

MacosPasskeyServicer::~MacosPasskeyServicer() {
  if (objcBridge) {
    CFRelease(objcBridge);
  }
}

/* AES key helps manage the auth flow by checking whether or not the user requires
*  passkey registration or authentication and is stored/retrieved via keychain */

// validate methods
// passkey must be registered if no AES key is found in keychain
void MacosPasskeyServicer::registerPasskey() {
  /*
   * if (!AES_key) {
   *   // generate AES_key
   *   // store AES_key in keychain
   *   [(__bridge PasskeyBridge*)objcBridge startRegistration]; // bridge objc registration method
   * }
   * else { this->authenticateWithPasskey(AES_key) }
   */

}

// can only be authenticated with passkey if AES key is found in keychain
void MacosPasskeyServicer::authenticateWithPasskey() {
  [(__bridge PasskeyBridge*)objcBridge startAuthentication];
}
