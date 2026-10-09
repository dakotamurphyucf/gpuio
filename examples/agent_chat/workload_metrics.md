# How `Workload_metrics` samples opt-in diagnostics

[README](README.md) · [Implementation](workload_metrics.ml)
· [Interface](workload_metrics.mli) · [Caller](application.md)

`Application.run` calls `start` only with `--workload-metrics`. This module is a
sampling task, not a Bonsai component, renderer, producer backend, or acceptance
test. Ordinary startup has no metrics task. It captures explicit Eio clock/stdout
capabilities from `env` and uses the application's root scope.

`App` is `Gpuio_eio.App`, `Scope` is `Gpuio_eio.Scope`, `Conversation` is the demo
runtime controller, `Document` is the public retained document adapter, `Source`
is `Gpuio.Text_source`, and `E` means `Bonsai.Effect`. There is no graph or
`let%arr`: existing conversation/workspace components own reactive state and views.
`Scope.start` runs an Eio fiber, and its result callback returns a deferred thunk
that unwraps errors. Application cancellation owns the infinite sampling loop.

Every 500 ms, after sleeping, it reads `App.diagnostics app`. It separately sums
`Source.byte_length` of the **last document** in each supplied conversation, using
zero when no document/source exists. `response_bytes` is therefore neither all
historical response bytes nor attachment/history/editor memory. Source lookup
reads retained values; it does not fetch backend content or traverse native views.

The task writes `GPUIO_CHAT_WORKLOAD`, elapsed milliseconds since sampler startup,
that byte sum, and a sexp of diagnostics through `Gpuio_eio.Output.write` with
explicit stdout. A streaming completion updates conversation/document state;
the next sample observes whichever source is current. Sampling is not an event
log and can miss intermediate peaks or changes between ticks. Output work is
itself part of the measured process and scheduler load.

The diagnostics include runtime/task/resource/transport accounting whose units
and lifetimes differ. This sampler contributes to its own task count. Source
reservations are conservative OCaml-side charges, not total process RSS, native
decoded caches, GPU allocation, or user-visible latency. Bridge drain counts can
include empty clock turns; accepted messages/events differ from attempts. Read
[diagnostic definitions](../../docs/design/runtime.md#read-only-runtime-diagnostics)
before comparing records. This module asserts no thresholds and prints no
completion/acceptance marker.

From the repository root using [development setup](../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/agent_chat/main.exe
_build/default/examples/agent_chat/main.exe --workload-metrics --full-motion
python3 scripts/test_agent_chat_combined.py
```

The app needs a native graphical session. Its workload flag also selects the
long fake streaming configuration in application setup; this module does not
configure or start that stream. Submit a prompt to produce activity and use
Cancel/close/quit to finish. There is no `workload_metrics.exe` and no sampler
process deadline; app shutdown cancels its task.

The separate combined harness requires macOS Accessibility and screen-capture
permission, drives multiple windows/artifacts, and owns its child lifetime.
Its assertions and painted-motion observations are separate from these log
snapshots; see README and [evidence](../../docs/evidence/agent-chat-m5.md#combined-streaming-large-artifacts-and-cleanup).
Neither sampling nor compilation establishes physical input or Linux GUI acceptance.

When adding metrics, preserve explicit capability/scope ownership, label sampled
versus exact counters, and define what each payload sum includes. Prefer bounded
sampling and external process measurements for whole-process CPU/RSS; avoid
turning diagnostic output into a user-facing application requirement.
