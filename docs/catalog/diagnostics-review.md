# Diagnostics and native observation helpers — OCH-41 / OCH-17

Reviewed 2026-10-04 against pinned GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Original `measure`, `observe`,
`test_support` and `macos_accessibility` modules are retained in
[the source manifest](sources/manifest.json). Their source presence is not a
performance result, screen-reader result or public OCaml automation API.

| Pinned behavior | Mapping and limitations |
| --- | --- |
| `Measure`, `measure`, `measure_if`, environment-enabled tracing | `App.stats`/`App.diagnostics`, runtime benchmarks and native instrumentation supply the relevant diagnostic paths. The Base helper measures one synchronous Rust closure and emits a tracing event; it is not imported as an OCaml profiler. Arbitrary Rust extensions can instrument their own work. GPUIO does not promise that setting `GPUI_MEASUREMENTS` installs a tracing subscriber or starts whole-app profiling. |
| `.test_support()` / `Observed<E>` | Native TestPlatform checks and actual AccessKit trees inspect production controls. Upstream's optional wrapper forwards native state and does not add a layout node; GPUIO does not expose its Rust `GlobalElementId` registry as an OCaml query service. Component developers can use native tests. This development API is distinct from user-facing accessibility semantics. |
| Paint snapshots, scoped lookup, ambiguity checks, focus-binding assertion | Read-only test evidence: bounds/role/value/disabled/selected/expanded/focus snapshots represent their recorded frame. Missing focus bindings and absent disabled flags are not success. No synthetic test-only state should override a native control or be used to prove OS behavior. |
| Registration drop and per-window observation registry | Native test resources must retire with their real owners. Existing host/controller lifecycle and document ownership tests provide scoped release evidence; they are not whole-process/GPU-memory measurements. |
| macOS `install_window_hit_test_forwarder` | Upstream Root installs an `NSWindow.accessibilityHitTest:` forwarder to its content view in non-test builds. GPUIO now calls this existing helper when creating macOS windows, after physically reproducing that screen-point queries returned AXWindow instead of the editor. The [two-window regression](../evidence/window-accessibility-och17.md) now requires and obtains exact editor identity at its visible screen point. The [control/overlay follow-up](../evidence/form-label-point-routing-och17.md) also passes 21 exact screen-point hits. Full catalog geometry and VoiceOver remain separate qualification; this is not a blanket accessibility pass. |

## Public counters and evidence

`App.diagnostics` is a UI-domain snapshot of registered windows/resources,
pending requests, queued commands, scopes/tasks/cleanups and conservative encoded
source reservations. Native command-queue bytes/high-water marks exclude work
already executing and output queues. Source reservations exclude decoded caches,
application models and GPU allocations. Sampling does not request a frame.
The Runtime gallery makes those distinctions visible.

`test/runtime/scope_test.ml` verifies task-versus-queued-result/cancellation
accounting. Resource registry, mailbox and document lifetime tests exercise their
own bounds. The [selection checkpoint](../evidence/window-selection-och41.md)
records the current full OCaml/native suite. These are not fresh macOS process
memory or latency benchmarks. [Document ownership evidence](../evidence/document-resources-och17.md)
and [runtime measurements](../evidence/runtime-och9.md) retain their limits.

The existing [macOS focus fix](../evidence/window-accessibility-och17.md) verifies
key-window/focused-node behavior; it does not by itself test point-based AX lookup,
VoiceOver navigation or speech. Keep those gates distinct. macOS automation needs
actual desktop access and Linux graphical qualification remains in deferred
OCH-47; neither is replaced by compilation or TestPlatform snapshots.

Remaining diagnostics acceptance: named-hardware workload budgets, current
process/GPU/resource and idle measurements, real AX hit-testing/VoiceOver,
startup/chart-delay investigation and reproducible release artifacts. This row
remains open; a counter API or a root-module inventory does not complete OCH-17.
