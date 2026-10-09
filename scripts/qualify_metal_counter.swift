// No windows, renderer or GUI activation. Calibrate the Metal resource counter
// and the ownership boundary proposed for the separate lifecycle audit.
import AppKit
import Metal
import QuartzCore

enum Failure: Error { case unavailable, layerDevice, retainedView, allocation, counter, release }

func qualify() throws -> [String: Any] {
    guard let device = MTLCreateSystemDefaultDevice() else { throw Failure.unavailable }
    weak var weakView: NSView?
    weak var weakLayer: CAMetalLayer?
    let observedDevice: any MTLDevice = try autoreleasepool {
        let view = NSView(frame: NSRect(x: 0, y: 0, width: 10, height: 10))
        let layer = CAMetalLayer()
        layer.device = device
        view.wantsLayer = true
        view.layer = layer
        weakView = view
        weakLayer = layer
        guard let observed = (view.layer as? CAMetalLayer)?.device,
              observed.registryID == device.registryID else { throw Failure.layerDevice }
        return observed
    }
    // Core Animation can retain a layer in an implicit transaction until the
    // next run-loop turn; settle that transaction before auditing ownership.
    CATransaction.flush()
    let retirementDeadline = Date().addingTimeInterval(2)
    while (weakView != nil || weakLayer != nil) && Date() < retirementDeadline {
        autoreleasepool { RunLoop.current.run(until: Date().addingTimeInterval(0.01)) }
    }
    guard weakView == nil, weakLayer == nil else {
        fputs("view_released=\(weakView == nil) layer_released=\(weakLayer == nil)\n", stderr)
        throw Failure.retainedView
    }
    let before = observedDevice.currentAllocatedSize
    let requested = 8 * 1024 * 1024
    let during: Int = try autoreleasepool {
        guard let buffer = observedDevice.makeBuffer(length: requested, options: .storageModeShared)
        else { throw Failure.allocation }
        memset(buffer.contents(), 0x5a, requested)
        let value = observedDevice.currentAllocatedSize
        guard value >= before + requested else { throw Failure.counter }
        return withExtendedLifetime(buffer) { value }
    }
    let deadline = Date().addingTimeInterval(2)
    while observedDevice.currentAllocatedSize > before && Date() < deadline {
        Thread.sleep(forTimeInterval: 0.01)
    }
    let after = observedDevice.currentAllocatedSize
    guard after <= before else { throw Failure.release }
    return ["complete": true, "device_name": observedDevice.name,
            "registry_id": String(observedDevice.registryID),
            "requested_buffer_bytes": requested, "before_bytes": before,
            "held_bytes": during, "released_bytes": after,
            "view_released": weakView == nil, "layer_released": weakLayer == nil,
            "os": ProcessInfo.processInfo.operatingSystemVersionString,
            "scope": "isolated device-counter/ownership calibration; no GPUI workload or presentation measurement"]
}

do {
    let data = try JSONSerialization.data(withJSONObject: qualify(), options: [.prettyPrinted, .sortedKeys])
    print(String(decoding: data, as: UTF8.self))
} catch {
    fputs("METAL_COUNTER_CALIBRATION_FAILED: \(error)\n", stderr)
    exit(1)
}
