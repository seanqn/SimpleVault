#include "nsstatus_bridge.h"
#include <AppKit/AppKit.h>
#include <Cocoa/Cocoa.h>
#include <QScreen>
#include <QFile>

@interface StatusBarHandler : NSObject
@property (nonatomic, strong) NSStatusItem *statusItem;
@property (nonatomic, assign) AppStatusBar *bridge;

- (instancetype)initWithBridge:(AppStatusBar *)bridge;
- (void)invalidate;
- (void)onStatusBarItemClicked:(id)sender;
@end

@implementation StatusBarHandler

- (instancetype)initWithBridge:(AppStatusBar *)bridge {
    self = [super init];
    if (self) {
        _bridge = bridge;
        _statusItem = [[NSStatusBar systemStatusBar] statusItemWithLength:NSVariableStatusItemLength]; // variable width

        NSStatusBarButton *button = _statusItem.button;
        if (button) {
            button.target = self;
            button.action = @selector(onStatusBarItemClicked:);
            button.title = @"Vault";
            [button sendActionOn:(NSEventMaskLeftMouseUp | NSEventMaskRightMouseUp)];
        }
    }

    return self;
}

- (void)invalidate {
    _bridge = nullptr;

    if (_statusItem) {
        NSStatusBarButton *button = _statusItem.button;
        if (button) {
            button.target = nil;
            button.action = nil;
            button.image = nil;
            button.title = @"";
        }
        [[NSStatusBar systemStatusBar] removeStatusItem:_statusItem];
        _statusItem = nil;
    }
}

- (void)dealloc {
    [self invalidate];
}

- (void)onStatusBarItemClicked:(id)sender {
    if (!_bridge) return;

    NSStatusBarButton *button = self.statusItem.button;
    if (!button) return;

    // gets button frame in window coords and converts for screen coords
    NSRect buttonInWindow = [button convertRect:button.bounds toView:nil];
    NSRect buttonInScreen = [button.window convertRectToScreen:buttonInWindow];

    // converts from cocoa primary screen coords (bottom left origin) to qt (top left origin)
    NSScreen *primaryScreen = [NSScreen screens].firstObject;
    if (!primaryScreen) return;

    CGFloat primaryScreenHeight = NSMaxY(primaryScreen.frame);
    // CGFloat primaryScreenHeight = primaryScreen ? primaryScreen.frame.size.height : 0;

    // button to anchor in bottom center (qt coords)
    qreal qtX = buttonInScreen.origin.x;
    qreal qtY = primaryScreenHeight - NSMaxY(buttonInScreen);
    // qreal qtY = primaryScreenHeight - (buttonInScreen.origin.y + buttonInScreen.size.height);
    qreal width = buttonInScreen.size.width;
    qreal height = buttonInScreen.size.height;

    QPoint anchor(qtX + (width / 2.0), qtY + height);
    QRect rect(qtX, qtY, width, height);

    QScreen *targetScreen = QGuiApplication::screenAt(anchor);
    if (!targetScreen) {
        targetScreen = QGuiApplication::primaryScreen();
    }

    emit _bridge->statusItemClicked(anchor, rect, targetScreen);
}

@end

// bridge methods

// warnings for constructor and deconstructor: __bridge_ casts have no effect when not using ARC. should not be an issue for mac only builds?
AppStatusBar::AppStatusBar(QObject *parent)
    : QObject(parent) {
    StatusBarHandler *handler = [[StatusBarHandler alloc] initWithBridge:this];
    m_objcHandler = (__bridge_retained void *)handler;
}

AppStatusBar::~AppStatusBar() {
    if (m_objcHandler) {
        StatusBarHandler *handler = (__bridge_transfer StatusBarHandler *)m_objcHandler;
        m_objcHandler = nullptr;
        [handler invalidate];
    }
}

void AppStatusBar::setButtonTitle(const QString &title) {
    StatusBarHandler *handler = (__bridge StatusBarHandler *)m_objcHandler;
    if (handler && handler.statusItem.button) {
        handler.statusItem.button.title = title.toNSString();
    }
}

// clang warns of potential leak here, but objc deconstructor (invalidate()) cleans up all button members if exists
void AppStatusBar::setButtonIcon(const QString &resourceFPath) {
    StatusBarHandler *handler = (__bridge StatusBarHandler *)m_objcHandler;
    if (handler && handler.statusItem.button) {
        QFile file(resourceFPath);
        if (file.open(QIODevice::ReadOnly)) {
            QByteArray data = file.readAll();
            NSData *nsData = [NSData dataWithBytes:data.constData() length:data.size()];
            NSImage *image = [[NSImage alloc] initWithData:nsData];
            [image setTemplate:YES];
            handler.statusItem.button.image = image;
        }
    }
}