# Repeated window/resource retirement — OCH-17

Status: collector and public workload implemented; short macOS functional check
passes. Full repeated memory measurements and native/physical qualification are
not established by this initial checkpoint.

The [workload and collector](../../examples/performance_lifecycle/README.md)
run three warm-up then 30 measured cycles in one application process. Each window
owns an exact streamed Markdown source, a unique decoded 256×256 image, a retained
canvas with a native command acknowledgement, and an asynchronous extension.
After window-scope cancellation, resource and request counters reach zero and
scope/task/cleanup counts return to the one-root/one-worker baseline. Following
two seconds of settling, the child waits for a numbered collector acknowledgement
so `ps` can sample resident memory before another window allocates. There is no
forced GC or cache purge.

This measures acknowledged application registrations, not a census of every
native entity/cache, GPU allocation or physical footprint. A native render
callback and document publication do not prove pixels or parser completion.
The exact limits and required native/physical evidence remain in the
[predeclared qualification plan](../design/performance-qualification.md).

Local macOS 14.5 arm64 / M1 Max checks pass:

- Optimized lifecycle and table-diagnostics build plus repository `@fmt`.
- Four collector tests covering all-cycle evidence, leaks/missing samples,
  last-ten baseline limits (including an intermediate peak), incremental records,
  sample-before-ack ordering, and exact-child cleanup on sampling failure.
- Ten existing chart collector tests, including real non-GUI child failure,
  timeout, interruption and resource accounting, still pass after the optional
  checkpoint callback was added.
- Actual macOS smoke001: one warm-up plus three measured open/exercise/close
  cycles, image dimensions/source content/native canvas/extension observations,
  zero resource/request/queue counts after every close, and automatic child exit.

Smoke checkpoint RSS is 106,872,832 / 113,770,496 / 117,276,672 / 120,881,152 bytes;
whole-process peak RSS is 120,995,840 bytes. The small sample is growing and must
not be described as a demonstrated plateau. Smoke does not satisfy the full
30-cycle or final-ten memory target. No OS-level native-entity leak conclusion
follows from zero application registrations.

Artifacts in `scratch/agents/root-20261004-resumed/`:
`lifecycle-build-00{1,2}.log`, `lifecycle-report-tests-001.log`,
`lifecycle-collector-tests-001.log`, `lifecycle-smoke-001/`. The first build caught
a reserved OCaml 5.3 identifier and produced no accepted executable; the second
passes after renaming it. Raw reports retain executable hash and checkout state.
Associate full runs with their preserved source build before declaring acceptance.
