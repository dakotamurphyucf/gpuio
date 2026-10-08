# Application-request teardown repair — OCH-17

2026-10-08, physical Mac host (macOS 14.5 arm64), base `8da5c452`.
This is deterministic OCaml runtime evidence using real Eio/native allocation
and injected lifecycle replies; no native GUI or OS service is exercised.

## Reproduced failure

The runtime already documents attempt-all cleanup for window requests. Review of
File_dialog, Desktop, Clipboard and Notification against `App` admission/completion
paths found that the same guarantee was not implemented for application requests:

- On `Stopped`, desktop completions ran with an unprotected `Map.iter`. If its
  first callback raised, the second desktop callback was lost from the detached
  map, and ten requests in the other five families remained pending. The exception
  also skipped subsequent registry cleanup in that handler.
- `dispose_runtime` silently cleared pending application maps. Unlike window
  requests, these effects did not receive a terminal result on worker disposal.

The regression admits two requests through each existing application transport:
desktop, notifications, assets, documents, charts and canvases. The first desktop
and first chart completion intentionally raise different exceptions. Before repair:

```text
(Stopped true (1 0 0 0 0 0) 10)
(Dispose false (0 0 0 0 0 0) 0)
```

Fields are path, whether the first completion exception escaped, callback counts
by family, and remaining map entries. The first development attempt failed to
compile because the fixture used the reserved OCaml keyword `effect` as a variable;
that fixture mistake is retained separately from the behavioral failure. The
second run demonstrated terminal starvation but stopped at a later test assertion;
the third records both paths above. No expectations were promoted.

## Repair and verified contract

`complete_application_requests` detaches all six maps together, then uses the
existing `finish_cleanup` helper to attempt every independent completion with
`Closed`. A callback cannot replay those requests by reentering teardown. Native
`Stopped` also attempts each registry close independently. Worker disposal sets
`stopping` before invoking cleanup and uses the same completion helper instead of
silently clearing maps. Window/inbox/scope cleanup retains its existing policy.

Both paths now report:

```text
(Stopped true (2 2 2 2 2 2) 0)
(Dispose true (2 2 2 2 2 2) 0)
```

The first exception is preserved despite a later failing completion, and repeated
retirement does not change counts. A second test reenters teardown from a desktop
callback, attempts new work (immediately Closed), then injects duplicate/late
native replies. It verifies three expected completions only and empty maps for
both terminal paths. Existing window exception-cleanup and native-submission-race
regressions also run. No request limit, wire shape, native code or public signature
changes. Higher-level scope cancellation may suppress delivery to retired resource
owners; calling a transport completion is not a promise to finish asynchronous
application work after shutdown.

## Validation and artifacts

Using the repository's isolated stock OCaml 5.3/Bonsai v0.17 toolchain:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @lib/eio/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @lib/eio/runtest @test/runtime/runtest @test/desktop/runtest @test/notification/runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
git diff --check
```

The focused suites and full incremental test/format aliases pass. The tests use
bounded five-second Eio harnesses; they neither open windows nor contact OS
notification/file services. The [archive](application-teardown-och17/reports.tar.gz)
contains the before source, final source and exact attempt logs; every entry is
verified against the [manifest](application-teardown-och17/manifest.json).

The [API guide](../api-compatibility.md#desktop-service-lifetimes) now summarizes
service admission, readiness/backpressure, window versus application ownership,
and the difference between OS acceptance and visible delivery. This closes a
specific lifecycle defect, not the full API, macOS OS-service or release gates.
Foundation 37789987337 predates this repair; current-source hosted/Linux validation
remains required. The original scrolling-overlap report is unrelated and open.
