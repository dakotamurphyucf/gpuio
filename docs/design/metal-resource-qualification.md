# Metal resource accounting for the lifecycle audit

OCH-17 design, 2026-10-05. This supplements the completed RSS, physical footprint,
IOSurface and GPUI entity audits. It does not replace them or measure presentation.

## Counter and ownership

Apple exposes `MTLDevice.currentAllocatedSize` as the memory used by that device's
resources, in bytes. Read it from the renderer's actual device; never substitute
an independently created default device when qualifying a GPUI window.
[Apple API](https://developer.apple.com/documentation/metal/mtldevice/currentallocatedsize).
The installed SDK's `MTLDevice.h` declares this counter and `registryID`; its
registry identifier identifies the GPU across tasks, not a resource allocation.

The pinned GPUI macOS raw window handle borrows its live NSView. That view's
backing layer comes from the renderer's Metal layer. On the main thread, verify
the layer is a CAMetalLayer, obtain its device and retain only that device for
the audit. Release the view/layer references immediately. Holding the device must
not retain a window, view, layer, GPUI entity or extension event route. Record its
registry ID and reject device changes during a run rather than comparing unlike
devices. Missing/non-Metal handles are unavailable, never zero-byte observations.

This route needs no GPUI fork change. The integration belongs to the dedicated
qualification backend, whose tracking overhead is excluded from responsiveness
runs. Ordinary applications must not acquire new timers, probes or diagnostics.

## Integration contract

Extend the resource-audit component with an explicitly requested Metal-memory
option and a versioned schema shared by its Rust/OCaml halves. On macOS, attach
the device probe to each successive owned window; require the same device identity.
Unsupported backends must reject requested qualification or record unavailable;
they cannot produce a passing Metal report. Required Linux unit/consumer checks
continue to cover ordinary resource-audit behavior.

During the live window, capture a bounded observed maximum when the audit component
renders, without invalidating the view. Label it a sampled maximum, not an exact
peak. After the existing one-second closed-window settlement and entity check,
record the device counter before permitting the next cycle. Preserve the existing
application-retirement and native-entity acknowledgements. The collector must also
require the matching Metal record when this option is requested, rejecting missing,
duplicate, reordered, wrong-device or malformed records. No event is sent back
through a retired component.

Use the existing three warmups plus thirty measured cycles, with three independent
optimized runs. Before these measurements, fix a final-ten closed-checkpoint growth
guardrail of 64 MiB, reporting all values, full-run range and warmup comparisons.
This guardrail detects continued growth at the scale of this small reference
window; it is not a general GPU memory allowance for applications. Exact owned
registrations and new GPUI entities must still retire, and the separate closed
IOSurface check must still pass. Investigate any persistent growth or plateau;
passing one guardrail does not establish absence of every leak.

Retain raw records, hardware/OS, device ID, source/binary revisions, build profile
and cancellation/cleanup outcomes. Device-reported allocations are not complete
process memory, compositor ownership, dedicated VRAM residency or a census of all
driver allocations. No FPS, GPU completion or input-to-photon claim follows.

## Calibration before integration

`scripts/qualify_metal_counter.swift` creates no window and performs no GUI
activation. It verifies that retaining a device obtained through a disposable
NSView/CAMetalLayer does not retain either object.
Implicit Core Animation transactions must first settle through the normal run
loop; the calibration bounds that wait at two seconds and fails if either object
remains. The lifecycle audit uses its existing closed-window settlement instead.
The calibration then allocates and writes an
8 MiB buffer, requires the counter to increase by at least that amount while the
buffer is held, and requires it to return to its prior level after release.
Unavailable devices or failed sensitivity/release checks fail the tool.

This isolated calibration validates the proposed counter/ownership mechanism.
It does not prove that the GPUI device has been captured, that the lifecycle
workload stays bounded, or that actual frames were presented. Those integration
and workload results must be recorded separately before acceptance.
