// Disposable Launch Services document consumer used by the macOS integration
// test. No UI or user data access; records the exact requested fixture path.
#import <Cocoa/Cocoa.h>

@interface Receiver : NSObject <NSApplicationDelegate>
@end
@implementation Receiver
- (void)applicationDidFinishLaunching:(NSNotification *)notification {
    (void)notification;
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, 5 * NSEC_PER_SEC), dispatch_get_main_queue(), ^{
        [NSApp terminate:nil];
    });
}
- (void)application:(NSApplication *)application openURLs:(NSArray<NSURL *> *)urls {
    NSURL *receipt = [[[NSBundle mainBundle].bundleURL URLByDeletingLastPathComponent]
        URLByAppendingPathComponent:@"opened-path.txt"];
    NSString *path = urls.firstObject.path;
    [path writeToURL:receipt atomically:YES encoding:NSUTF8StringEncoding error:nil];
    dispatch_after(dispatch_time(DISPATCH_TIME_NOW, NSEC_PER_SEC / 4), dispatch_get_main_queue(), ^{
        [application terminate:nil];
    });
}
@end
int main(void) {
    @autoreleasepool {
        NSApplication *app = [NSApplication sharedApplication];
        Receiver *receiver = [Receiver new];
        app.delegate = receiver;
        [app run];
    }
    return 0;
}
