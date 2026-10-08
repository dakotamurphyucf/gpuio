// Independent AppKit oracle. Command-line defaults affect only this process.
import AppKit

let app = NSApplication.shared
app.setActivationPolicy(.prohibited)
print(NSScroller.preferredScrollerStyle == .overlay ? "Auto_hide" : "Always_visible")
