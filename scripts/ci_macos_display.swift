// Give hosted GUI fixtures their documented desktop space without changing
// permanent display preferences. --check is read-only; --apply is CI-only.
import AppKit
import CoreGraphics
import Foundation

let minimumWidth = 1440
let minimumHeight = 1000

func describe() -> Bool {
    let id = CGMainDisplayID()
    let bounds = CGDisplayBounds(id)
    let screen = NSScreen.screens.first {
        ($0.deviceDescription[NSDeviceDescriptionKey("NSScreenNumber")] as? NSNumber)?.uint32Value == id
    }
    let visible = screen?.visibleFrame ?? .zero
    print("MACOS_DISPLAY id=\(id) bounds=\(bounds) visible=\(visible) scale=\(screen?.backingScaleFactor ?? 0)")
    return bounds.width >= CGFloat(minimumWidth) && bounds.height >= CGFloat(minimumHeight)
        && visible.width >= 1400 && visible.height >= 900
}

func checked(_ result: CGError, _ operation: String) throws {
    if result != .success {
        throw NSError(domain: "GPUIO display setup", code: Int(result.rawValue),
                      userInfo: [NSLocalizedDescriptionKey: "\(operation): CGError \(result.rawValue)"])
    }
}

do {
    let arguments = Array(CommandLine.arguments.dropFirst())
    guard arguments == ["--check"] || arguments == ["--apply"] else {
        throw NSError(domain: "GPUIO display setup", code: 1,
                      userInfo: [NSLocalizedDescriptionKey: "Use --check or --apply (GitHub Actions only)"])
    }
    if arguments == ["--apply"] && ProcessInfo.processInfo.environment["GITHUB_ACTIONS"] != "true" {
        throw NSError(domain: "GPUIO display setup", code: 1,
                      userInfo: [NSLocalizedDescriptionKey: "Refusing display changes outside GitHub Actions"])
    }
    let id = CGMainDisplayID()
    let modes = (CGDisplayCopyAllDisplayModes(id, nil) as? [CGDisplayMode] ?? [])
        .filter { $0.isUsableForDesktopGUI() }
    print("MACOS_DISPLAY_MODES " + modes.map {
        "\($0.width)x\($0.height) pixels=\($0.pixelWidth)x\($0.pixelHeight)"
    }.joined(separator: "; "))
    if !describe() {
        guard arguments == ["--apply"] else {
            throw NSError(domain: "GPUIO display setup", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "GUI fixtures require at least 1440x1000 points with 1400x900 usable"])
        }
        let current = CGDisplayCopyDisplayMode(id)
        let density = Double(current?.pixelWidth ?? 1) / Double(current?.width ?? 1)
        var candidates = modes.filter { $0.width >= minimumWidth && $0.height >= minimumHeight }
        candidates.sort { first, second in
            let a = abs(Double(first.pixelWidth) / Double(first.width) - density)
            let b = abs(Double(second.pixelWidth) / Double(second.width) - density)
            return a == b ? first.width * first.height < second.width * second.height : a < b
        }
        guard let mode = candidates.first else {
            throw NSError(domain: "GPUIO display setup", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "No sufficiently large desktop mode; available modes are logged above"])
        }
        var configuration: CGDisplayConfigRef?
        try checked(CGBeginDisplayConfiguration(&configuration), "Begin configuration")
        let configured = CGConfigureDisplayWithDisplayMode(configuration, id, mode, nil)
        if configured != .success {
            CGCancelDisplayConfiguration(configuration)
            try checked(configured, "Select desktop mode")
        }
        // ForSession survives this helper's exit without saving permanent settings.
        try checked(CGCompleteDisplayConfiguration(configuration, .forSession), "Apply session mode")
        let deadline = Date().addingTimeInterval(5)
        var ready = describe()
        while !ready && Date() < deadline {
            RunLoop.current.run(until: Date().addingTimeInterval(0.1))
            ready = describe()
        }
        guard ready else {
            throw NSError(domain: "GPUIO display setup", code: 1,
                          userInfo: [NSLocalizedDescriptionKey: "Configured desktop did not provide the required usable area"])
        }
    }
    print("MACOS_DISPLAY_READY")
} catch {
    fputs("\(error.localizedDescription)\n", stderr)
    exit(1)
}
