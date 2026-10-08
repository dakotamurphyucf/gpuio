// Independent AppKit oracle. Command-line defaults affect only this process.
import AppKit

let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
func style() -> String {
    NSScroller.preferredScrollerStyle == .overlay ? "Auto_hide" : "Always_visible"
}
let startup = style()
app.finishLaunching()
// AppKit resolves automatic pointing-device policy asynchronously. A cold query
// on the hosted Mac reports overlay before its event loop resolves legacy style.
// This is a fixed initialization interval, not a retry until an expected style.
RunLoop.current.run(until: Date(timeIntervalSinceNow: 0.5))
let result: [String: Any] = [
    "startup": startup,
    "settled": style(),
    "argument_default": UserDefaults.standard.string(forKey: "AppleShowScrollBars") ?? "unset",
    "initialization_seconds": 0.5,
]
print(String(data: try! JSONSerialization.data(withJSONObject: result, options: [.sortedKeys]), encoding: .utf8)!)
