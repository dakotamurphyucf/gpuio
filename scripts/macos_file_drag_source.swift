// Independent AppKit source for cross-process file-drop qualification.
// Only command-line fixture URLs are offered; no Finder or clipboard writes.
import AppKit

final class FileSource: NSView, NSDraggingSource {
    let urls: [URL]
    var completed = 0

    init(urls: [URL]) {
        self.urls = urls
        super.init(frame: NSRect(x: 0, y: 0, width: 190, height: 110))
        setAccessibilityElement(true)
        setAccessibilityRole(.group)
        setAccessibilityLabel("Fixture file source")
    }

    required init?(coder: NSCoder) { fatalError("No archives") }

    override func draw(_ dirtyRect: NSRect) {
        NSColor.windowBackgroundColor.setFill()
        bounds.fill()
        ("Drag fixture files" as NSString).draw(
            at: NSPoint(x: 18, y: 45),
            withAttributes: [.font: NSFont.systemFont(ofSize: 16),
                             .foregroundColor: NSColor.labelColor])
    }

    override func mouseDown(with event: NSEvent) {
        let position = convert(event.locationInWindow, from: nil)
        let items = urls.map { url -> NSDraggingItem in
            let item = NSDraggingItem(pasteboardWriter: url as NSURL)
            item.setDraggingFrame(NSRect(x: position.x, y: position.y, width: 32, height: 32),
                                  contents: NSImage(named: NSImage.multipleDocumentsName))
            return item
        }
        let session = beginDraggingSession(with: items, event: event, source: self)
        session.animatesToStartingPositionsOnCancelOrFail = false
    }

    func draggingSession(_ session: NSDraggingSession,
                         sourceOperationMaskFor context: NSDraggingContext) -> NSDragOperation {
        return .copy
    }

    func draggingSession(_ session: NSDraggingSession, endedAt screenPoint: NSPoint,
                         operation: NSDragOperation) {
        completed += 1
        print("FILE_SOURCE_ENDED count=\(completed) operation=\(operation.rawValue)")
        fflush(stdout)
    }
}

let arguments = Array(CommandLine.arguments.dropFirst())
guard !arguments.isEmpty, arguments.allSatisfy({ $0.hasPrefix("/") }) else {
    fatalError("Supply absolute fixture paths")
}
let app = NSApplication.shared
app.setActivationPolicy(.regular)
let window = NSWindow(contentRect: NSRect(x: 20, y: 200, width: 190, height: 110),
                      styleMask: [.titled, .closable, .resizable], backing: .buffered, defer: false)
window.title = "GPUIO File Source Fixture"
window.contentView = FileSource(urls: arguments.map { URL(fileURLWithPath: $0) })
window.makeKeyAndOrderFront(nil)
app.activate(ignoringOtherApps: true)
app.run()
