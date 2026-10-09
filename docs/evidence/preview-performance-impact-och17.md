# Preview streaming/resource change-impact review

The owner’s focused qualification plan permits reuse of passing full trials after
reviewing subsequent changes and checking affected paths. This review compares
the streaming baseline `2562490f` and resource baseline `48a008cf` with the current
implementation `31bffeee`. Later changes through `87b43c73` affect documentation
and standalone CI probe tooling, not the workload, renderer or application runtime.

## Scope of intervening changes

| Change | Effect on these workloads and validation |
| --- | --- |
| Inherited list-metric cache invalidation | Adds one cached text/rem/scale record per native list, checked before layout. Streaming uses the list with constant metrics; the new full 10k-list comparison exercises the current shared path. Existing metric-change native regressions cover invalidation/anchor correctness. |
| Reactive scrollbar argument on managed adapters | Default `None` preserves the native configuration used by streaming. Key/lifecycle and clear/reapply regressions plus root/installed gallery checks cover the new option. Current streaming smoke checks default composition. |
| Accessibility focus forwarding | Changes OS focus queries, not editor mutation or input transport. Current native typing checks exercise real foreground routing and text observation. Existing two-window/native focus evidence remains scoped to that change. |
| Clipped document controls and cancelled native scroll phase | The typing workload uses plain `View.text`, not rich document controls, and does not send wheel events. Resource smoke exercises the ordinary document lifetime; focused document/scroll regressions cover the changed behavior separately. |
| Scope/application/force-close exception handling | Changes cancellation and shutdown ordering so a raising callback cannot strand unrelated work. Deterministic exception/reentrancy regressions cover those branches; current normal-close resource smoke checks native integration. No new recurring timer, retained history cache or GPU allocation policy was added. |

The stream model, key-dispatch driver, editor mutation path, profiler/presentation
collector, rendering clock and workload sizes are unchanged. No dependency or
compiler pin changed. Source review alone does not establish performance; it is
combined here with the targeted checks and the new full shared-list measurements.

## Current-source local checks

Both optimized executables rebuilt successfully using the isolated toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release -j2 \
  examples/performance_presented/streaming/main.exe examples/resource_audit/main.exe
```

The runs observed clean `87b43c73` on the same M1 Max / macOS 14.5 arm64 reference.
No compiler or second owned GUI ran during either test. Each child exited zero
and was reaped. The [raw archive](preview-performance-impact-och17/reports.tar.gz)
retains build/driver logs, reports, input-source recovery and physical-memory
records; all 32 members match the [manifest](preview-performance-impact-och17/manifest.json).

- **Streaming smoke passes:** four streams complete 80 updates each over four
  seconds; all 40 OS keyboard events produce the exact expected composer text and
  40 native input samples. Peak active history remains ten rows. Window/resource
  cleanup passes and the original input source is restored. Executable SHA-256
  `275a748c78fcaa5da2841e782f257aeb483a3fa2410ac371fc07437b7a3b4b60`.
- **Resource smoke passes:** one warmup and three measured window lifetimes retire
  application resources and pending work. No additional live native entities remain
  relative to the closed warmup baseline. All four presentation sessions stop,
  close and settle; one cycle's two zero-time callbacks are retirement evidence,
  not presentation acceptance. Closed Metal allocation settles at 2,490,368 bytes,
  with zero growth across the three measured closed checkpoints. Closed IOSurface
  accounting is zero. Executable SHA-256
  `99ac53559eff11b167105aea72dc3552d7c26eb8f6b4e048d88d5ee027f7e543`.

The archive preserves the exact commands. Streaming uses
`measure_streaming_typing.py --build-profile release --presentation --smoke`;
resource checks use `measure_resource_lifecycle.py --build-profile release
--native-entities --presentation --metal-memory --physical-memory
--check-closed-surfaces --smoke`, with the corresponding executable paths above.

## Evidence retained for the preview

Retain the [three full streaming/typing trials](presentation-streaming-current-och17.md)
and [three 33-cycle resource trials](presentation-retirement-och17.md#three-full-resource-repetitions)
at their original revisions. These short runs are regression checks, **not new
120-second timing or 30-cycle memory qualification**. Their reports correctly
retain smoke status. Combined with this change-impact review, the targeted
regressions and the [completed current list/instrumentation comparison](preview-list-comparison-och17.md),
they satisfy P2's focused preview evidence-reuse policy without repeating the old
four-workload timing matrix. This does not establish indefinite leak freedom,
complete OS/GPU memory accounting, or arbitrary 120 FPS application performance.

P1's normal-scroll assessment and the remaining notice/install/CI/release work are
separate. Broader qualification remains in OCH-164 and Linux desktop in OCH-47.
