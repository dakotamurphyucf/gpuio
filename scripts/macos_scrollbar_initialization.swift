import AppKit
import Foundation
func sample(_ phase: String) {
    let data: [String: Any] = ["phase": phase,
        "style": NSScroller.preferredScrollerStyle == .overlay ? "Auto_hide" : "Always_visible",
        "default": UserDefaults.standard.string(forKey: "AppleShowScrollBars") ?? "unset"]
    print(String(data: try! JSONSerialization.data(withJSONObject: data, options: [.sortedKeys]), encoding: .utf8)!)
}
let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
sample("before-finish-launching")
app.finishLaunching()
sample("after-finish-launching")
RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.25))
sample("after-run-loop")
let scroll = NSScrollView(frame: NSRect(x: 0, y: 0, width: 100, height: 100))
scroll.hasVerticalScroller = true
sample("after-scroll-view")
