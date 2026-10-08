# Scope cleanup exception safety — OCH-17

2026-10-08, macOS 14.5 arm64, isolated OCaml 5.3/Bonsai v0.17 environment,
based on `929ef037`. This repairs OCaml lifecycle handling; no Rust, protocol,
dependency, OS-service or UI behavior is changed.

## Reproduced failure

`Scope.cancel` marked the scope inactive, then iterated children, tasks and
cleanup callbacks without exception isolation. If a descendant cleanup raised,
it skipped sibling cancellation and later cleanup/accounting. Repeating cancel
could not recover the work because the root was already inactive.

The pre-repair expect regression creates a root, two children, one grandchild,
five callbacks and a blocked sibling producer. The first grandchild callback
raises. Actual output records only that callback; the sibling remains active,
its producer unfinished, and shared statistics still report four scopes, one
task and three registered cleanups. A repeated cancellation leaves those counts
unchanged. The fixture explicitly cancels its stranded producer afterward so a
failing regression cannot hang its test switch.

## Repair and contract

Each scope now detaches child membership and retires its own scope count before
calling user code. It attempts every descendant and task cancellation, then
claims and clears its remaining cleanup batch before executing every callback.
The first failure is captured with its raw backtrace and re-raised after the
remaining independent work. It is not swallowed or converted to a success result.
Task accounting still waits for producer fibers to unwind, including cancellation.

The callback batch is captured after descendant cancellation. This preserves a
child cleanup's ability to unregister a pending ancestor callback. An additional
regression caught the first candidate repair taking that snapshot too early;
its failure is retained, and implementation ordering was corrected rather than
promoting the expectation. Reentrant cancel and unregister remain idempotent.
No global atomic cancellation of all sibling scopes is promised.

Cleanup callbacks must still be non-raising, nonblocking and free of I/O. Attempting
other cleanup after an exception is defensive containment; it cannot make a
blocked callback return or forcibly terminate protected producer cleanup.

## Validation

Three expect cases cover:

- A failing grandchild and later failing sibling: all five callbacks run once,
  every scope becomes inactive, the producer finishes, final counts reach zero
  and the first failure remains the observed exception.
- Reentrant cancellation, rejected late child creation, queued-stream suppression,
  repeated unregister/cancel, surviving peer completion and a fresh replacement
  scope under the still-live parent.
- A child unregistering a pending ancestor callback without duplicate invocation
  or incorrect cleanup accounting.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/runtime/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt
```

The targeted runtime suite and full incremental OCaml test/format aliases pass.
No expect output was promoted. These deterministic scheduler/lifecycle tests
use Eio's mock backend; they do not provide OS-service, native GUI, performance,
VoiceOver or Linux qualification. OCH-17/OCH-41 and the whole API/release review
remain open.

The [verified archive](scope-cancellation-och17/reports.tar.gz) retains the original
failure, intermediate compatibility regression, final passing logs and exact
scope/interface/test sources. Its [manifest](scope-cancellation-och17/manifest.json)
records hashes/sizes for eight artifacts (24,259 bytes). Example inventory
(432 sources/268 groups), catalog audit and whitespace checks also pass.
Hosted execution of this revision remains pending.
