// Qualify Apple's drawable presentation callback on an actual owned window.
// This is an API/clock probe, not GPUI workload or photon-latency evidence.
import AppKit
import Metal
import QuartzCore

final class Records: @unchecked Sendable {
    private let lock = NSLock()
    private var rows: [Int: [String: Any]] = [:]
    func add(_ sequence: Int, _ values: [String: Any]) {
        lock.lock()
        defer { lock.unlock() }
        precondition(sequence >= 0 && sequence < 120)
        rows[sequence, default: [:]].merge(values) { _, new in new }
    }
    func snapshot() -> [[String: Any]] {
        lock.lock()
        defer { lock.unlock() }
        return rows.keys.sorted().map { rows[$0]! }
    }
}

final class Probe: NSObject, NSApplicationDelegate, NSWindowDelegate {
    let records = Records()
    var window: NSWindow?
    var layer: CAMetalLayer?
    var queue: (any MTLCommandQueue)?
    var timer: Timer?
    var sent = 0
    var startupWaitTicks = 0
    var deadline = Date.distantPast
    var finished = false
    var deviceName = ""

    func applicationDidFinishLaunching(_ notification: Notification) {
        guard let device = MTLCreateSystemDefaultDevice(), let queue = device.makeCommandQueue() else {
            finish("No Metal device/queue")
            return
        }
        self.queue = queue
        deviceName = device.name
        let window = NSWindow(contentRect: NSRect(x: 200, y: 200, width: 640, height: 360),
                              styleMask: [.titled, .closable], backing: .buffered, defer: false)
        self.window = window
        window.isReleasedWhenClosed = false
        window.delegate = self
        window.title = "GPUIO · Metal presentation probe (closes automatically)"
        let view = NSView(frame: NSRect(x: 0, y: 0, width: 640, height: 360))
        let layer = CAMetalLayer()
        self.layer = layer
        layer.device = device
        layer.pixelFormat = .bgra8Unorm
        layer.framebufferOnly = true
        layer.presentsWithTransaction = false
        view.wantsLayer = true
        view.layer = layer
        window.contentView = view
        let label = NSTextField(labelWithString: "Measuring actual Metal presentation callbacks…")
        label.textColor = .white
        label.font = .systemFont(ofSize: 18, weight: .medium)
        label.frame = NSRect(x: 32, y: 164, width: 576, height: 30)
        view.addSubview(label)
        window.makeKeyAndOrderFront(nil)
        NSApp.activate(ignoringOtherApps: true)
        layer.contentsScale = window.backingScaleFactor
        layer.drawableSize = CGSize(width: 640 * window.backingScaleFactor,
                                    height: 360 * window.backingScaleFactor)
        deadline = Date().addingTimeInterval(12)
        timer = Timer.scheduledTimer(withTimeInterval: 1.0 / 60.0, repeats: true) { [weak self] _ in
            self?.tick()
        }
    }

    func tick() {
        guard !finished else { return }
        if Date() >= deadline { finish("Timed out waiting for all presentation/completion callbacks"); return }
        // Activation/occlusion notifications are asynchronous. Start the bounded
        // series only after the owned window is actually active and visible.
        if sent == 0 && (!NSApp.isActive || window?.occlusionState.contains(.visible) != true) {
            startupWaitTicks += 1
            return
        }
        if sent == 120 {
            let rows = records.snapshot()
            if rows.count == 120 && rows.allSatisfy({ $0["presented_s"] != nil && $0["gpu_end_s"] != nil }) {
                finish(nil)
            }
            return
        }
        autoreleasepool {
            guard let layer, let queue, let drawable = layer.nextDrawable(),
                  let buffer = queue.makeCommandBuffer() else {
                finish("Drawable/command-buffer acquisition failed")
                return
            }
            let sequence = sent
            sent += 1
            let pass = MTLRenderPassDescriptor()
            pass.colorAttachments[0].texture = drawable.texture
            pass.colorAttachments[0].loadAction = .clear
            pass.colorAttachments[0].storeAction = .store
            pass.colorAttachments[0].clearColor = MTLClearColor(red: 0.055, green: 0.13 + Double(sequence % 2)*0.005,
                                                              blue: 0.17, alpha: 1)
            guard let encoder = buffer.makeRenderCommandEncoder(descriptor: pass) else {
                finish("Cannot encode clear pass"); return
            }
            encoder.endEncoding()
            let records = self.records
            records.add(sequence, ["sequence": sequence, "drawable_id": UInt64(drawable.drawableID),
                                   "before_presented_s": drawable.presentedTime,
                                   "visible": window?.occlusionState.contains(.visible) == true,
                                   "active": NSApp.isActive])
            // Handlers capture only bounded records and sequence, never the
            // window/layer/drawable/command-buffer itself.
            drawable.addPresentedHandler { presented in
                records.add(sequence, ["presented_s": presented.presentedTime,
                                       "presentation_callback_host_s": CACurrentMediaTime()])
            }
            buffer.addCompletedHandler { completed in
                records.add(sequence, ["gpu_start_s": completed.gpuStartTime,
                                       "gpu_end_s": completed.gpuEndTime,
                                       "gpu_status": completed.status.rawValue,
                                       "completion_callback_host_s": CACurrentMediaTime()])
            }
            records.add(sequence, ["submit_before_s": CACurrentMediaTime()])
            buffer.present(drawable)
            buffer.commit()
            records.add(sequence, ["submit_after_s": CACurrentMediaTime()])
        }
    }

    func windowShouldClose(_ sender: NSWindow) -> Bool {
        finish("Owned probe window was closed before completion")
        return false
    }

    func finish(_ error: String?) {
        guard !finished else { return }
        finished = true
        timer?.invalidate()
        timer = nil
        let rows = records.snapshot()
        let times = rows.compactMap { $0["presented_s"] as? Double }
        var failures: [String] = error.map { [$0] } ?? []
        if rows.count != 120 || times.count != 120 { failures.append("Missing frame records") }
        for row in rows {
            guard let presented = row["presented_s"] as? Double,
                  let before = row["submit_before_s"] as? Double,
                  let callback = row["presentation_callback_host_s"] as? Double,
                  let gpuStart = row["gpu_start_s"] as? Double,
                  let gpuEnd = row["gpu_end_s"] as? Double else {
                failures.append("Incomplete frame record")
                continue
            }
            if ![presented, before, callback, gpuStart, gpuEnd].allSatisfy({ $0.isFinite }) {
                failures.append("Non-finite clock sample")
            }
            if presented <= 0 || presented < before || presented > callback {
                failures.append("Invalid presentation clock ordering at \(row["sequence"]!)")
            }
            if gpuStart <= 0 || gpuEnd < gpuStart || (row["gpu_status"] as? UInt) != MTLCommandBufferStatus.completed.rawValue {
                failures.append("Invalid GPU completion at \(row["sequence"]!)")
            }
            if row["visible"] as? Bool != true {
                failures.append("Probe window not visible at submission")
            }
        }
        if zip(times, times.dropFirst()).contains(where: { $0 >= $1 }) {
            failures.append("Presented timestamps are not strictly increasing in submitted order")
        }
        let drawableIDs = rows.compactMap { $0["drawable_id"] as? UInt64 }
        if Set(drawableIDs).count != 120 { failures.append("Missing or repeated drawable identities") }
        let report: [String: Any] = [
            "complete": failures.isEmpty, "failures": failures, "device": deviceName,
            "os": ProcessInfo.processInfo.operatingSystemVersionString,
            "requested_frames": 120, "submitted_frames": sent,
            "startup_wait_ticks": startupWaitTicks,
            "scope": "Metal API/host-clock qualification; not GPUI workload, GPU throughput or photon measurement",
            "frames": rows,
        ]
        do {
            let data = try JSONSerialization.data(withJSONObject: report, options: [.prettyPrinted, .sortedKeys])
            FileHandle.standardOutput.write(data)
            FileHandle.standardOutput.write(Data("\n".utf8))
        } catch { fputs("Cannot encode presentation report: \(error)\n", stderr) }
        window?.delegate = nil
        window?.close()
        window = nil
        layer = nil
        queue = nil
        exit(failures.isEmpty ? 0 : 1)
    }
}

let app = NSApplication.shared
let probe = Probe()
app.setActivationPolicy(.regular)
app.delegate = probe
app.run()
