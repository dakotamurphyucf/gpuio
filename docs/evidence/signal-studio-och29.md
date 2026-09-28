# Signal Studio combined workload and lifetime evidence (OCH-29)

Local Apple M1 Max, 32 GiB unified memory, macOS 14.5 arm64. Stock OCaml 5.3,
Rust 1.97.1, development build, unchanged pinned GPUI. Measured after `7a7b48a`
with the workload and opt-in component instrumentation in this checkpoint.
No concurrent GPUIO build or test ran. This is a diagnostic desktop run, not a
controlled release benchmark or a Linux GUI result.

The public Signal Studio application performs 384 desired workspace updates in
96 four-update UI turns. Every accepted final dataset is followed by a native
render callback. The chart contains four 24-point series, and the OCaml canvas
contains four interactive samples plus 160 decorative points and labels. The
application republishes both resources, updates document metadata and reconciles
the independently packaged native counter. This complements the separate
[10k/100k chart workload](chart-streaming-och40.md); it is not a large-data test.

| Measurement | Result |
| -- | --: |
| Publish/invalidation median / p95 | 1.12 / 2.08 ms |
| Update to render callback median / p95 / max | 24.87 / 43.53 / 74.19 ms |
| Whole child wall time | 4.37 s |
| User / system CPU | 1.76 / 0.38 s |
| Peak process RSS | 138,903,552 bytes (132.5 MiB) |
| Native accepted-command queue peak | 25,533 serialized bytes |
| Sampled source charge peak | 670,592 bytes |
| Source charge after each window closes | 335,296 bytes |
| Final registrations / source charge | 0 / 0 |
| Sampled pending request peak | 6 |
| Median submitted bytes / messages per batch | 25,614 / 12 |

Publish time includes constructing and submitting the four desired snapshots and
related application invalidation. Update time includes that work, native
publication/preparation and a render callback. Neither measurement proves a
physical screen presentation, FPS, isolated GPU duration or input latency. A burst
that returns to the already published dataset correctly produces no new chart
revision; the harness waits for publication equality rather than inventing a
required revision change. Ordinary source updates are coalesced before upload.
Registry charges are conservative admission accounting, distinct from process RSS;
queue bytes exclude executing work and output events. Sampled peaks may miss
between-sample states; the native queue high-water mark is tracked at admission.

## Explicit commands and repeated lifetime

Twelve same-generation counter commands set values 20 through 31. The component's
own native command hook records those values, and OCaml receives exact sequence
acknowledgements 1 through 12. Commands do not remount the component and are cleared
before a later window can replay them. Hidden/show presentation retains the same
instance. Each cycle then replaces the native generation and closes the window.
The next window reuses the application's authoritative workspace and source handles.

Across twelve cycles, all **24 native component instances** record exactly one
mount, unmount and component drop. Their separately reference-counted values also
record exactly one final drop, proving rendered callbacks release their references.
At most two such values overlap during generation replacement. A native frame's
last callback can drop just after OCaml observes window closure; the harness permits
at most one retired value at that boundary, requires it gone by the next rendered
sample, and requires zero values before the final workload checkpoint. It does not
promise synchronous callback destruction at receipt of `Window_closed`.

Every closed-window checkpoint has zero windows, one application scope, one
workload task, and one canvas/chart registration with the same source charge.
Explicit source release returns registrations/charges to zero before application
shutdown. This is stronger evidence than inferring native destruction solely from
OCaml counters. Traces belong to the example package and import only public SDK/std
APIs; `GPUIO_COUNTER_TRACE=1` is opt-in and leaves the wire/schema unchanged.

## Reproduction and related acceptance

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/signal_studio/main.exe
python3 scripts/measure_signal_studio.py --output scratch/signal-workload
python3 scripts/test_signal_studio.py --output scratch/signal-input
```

The harness owns/reaps its child, records revision/dirty state, toolchain, hardware,
full application log and JSON measurements. Its elapsed deadline includes startup
and closes the child on failure. Foreground render callbacks are used: the first
background experiment did not progress to layout on this desktop.

The separate AppKit walkthrough passes real canvas/chart/component input and
wide/compact/wide resizing. It checks exactly one accessible canvas, chart,
inspector region and each activity label; canvas widths are 700/490/700 logical
pixels. No inactive responsive branch remains exposed. Hidden inspector/component
checks and native container-query input tests provide additional input fencing
coverage. The [design matrix](../design/signal-studio.md) maps desktop, documents,
notifications and Full/Reduce motion to their own actual platform walkthroughs.
The same complete workload also passes in the staged independent consumer after
building against installed public libraries and the separately copied component.
Required hosted macOS/Linux gates remain pending, with Linux GUI tracked in OCH-17.

## Consolidated local validation

At `08423cd`, the complete Dune expect suites, 719 Rust workspace tests,
formatting, example builds and strict Clippy pass on the same macOS host. The
native chart GPU/input/AX checks, private-bus desktop/notification fixtures,
packaged desktop and notification OS walkthroughs, and all five Signal Studio
walkthroughs pass together with the following test-harness adjustments:

- Raise only the owned child window before screenshots and native pickers.
  A live occluded window was absent from the on-screen capture list.
- Wait for the native Save button to be enabled with stable geometry before one
  AX press. The previous immediate press could leave the presented sheet open.
- Observe the final accessible counter value before capturing the completed
  stream, independently of its earlier application-log event.

These changes do not alter document persistence or renderer behavior. The native
workload rerun records 4.06 seconds wall time, 1.70/0.37 seconds user/system CPU,
139,362,304 bytes peak RSS and update-to-render median/p95/max of
24.97/41.60/48.33 ms. All 384 desired updates, 96 render samples, twelve commands
and window cycles, 24 component/value lifetimes and final zero resource charges
pass again. These measurements have the same visibility and presentation limits
described above; the separate large-chart ledger preserves its latency outlier.

## Native automation transitions (2026-09-28)

Full run `36419908242` passed Signal Studio input, motion, workload and
notifications, but the desktop walkthrough timed out on its first counter action
after reset. Reset advances the component generation. Its old check waited for
zero while the old counter was already zero, so it could proceed before observing
the new generation. The test now increments to one before resetting and observes
zero afterward, preserving the native-generation fence. The complete local
document/picker/link/Finder walkthrough passes with this distinct transition.

A separate diagnostic run observed the completed closing spring but missed its
intermediate heights: it queried unrelated accessibility nodes before sampling.
The motion fixture now acquires the geometry node before activation and begins
sampling immediately afterward. Full/Reduce endpoint, intermediate-height,
interruption, sequence, shared-paint and cleanup assertions remain unchanged;
both local motion modes pass. These harness changes do not alter application or
framework behavior. Final required CI results are recorded in PR #14.

[Focused hosted run 36424019983](https://github.com/dakotamurphyucf/gpuio/actions/runs/36424019983)
passes these harness changes in sequence with Signal input, desktop, Full/Reduce
motion, bounded workload, Signal notifications and the Notification Lab. It
reuses previously built binaries; it does not replace final required builds,
independent-consumer checks or the full performance measurement wrapper.
