# How the native audit checks closed-window entity retention

[README](../../README.md) · [Source](lib.rs) · [Metal probe](metal.md)
· [OCaml instance](../../ocaml/gpuio_resource_audit.md)

This qualification-only extension uses pinned SDK/GPUI internals with
`leak-detection`. It has no production metrics hook, OCaml callback, Bonsai graph,
or synchronous cross-language event route. Its app-owned state deliberately
survives component retirement to inspect the closed window.

`Properties::decode` accepts the paired five-byte configuration, including
immutable warmup/measurement counts, next cycle, Metal and presentation requirements.
`Factory::descriptor` names schema version 3/fingerprint; command validation
always fails. `mount` calls `register`, then returns `Component`. Update accepts
only byte-equivalent configuration. Render optionally samples Metal and returns
an empty div; the component is instrumentation, not a visible audit dashboard.

`Audit` is a GPUI Global holding configuration, optional WindowId, checked count,
baseline, close subscription, one pending task, failure flag, optional device
probe and optional presentation session. It holds no strong Window/Entity/EventSink. `register` requires cycle 1 for
initialization, installs `on_window_closed`, then enforces sequential next cycle,
matching configuration, and no live audit window. Metal capture must use the
same renderer device. Taking the previous completed task prevents accumulating
historical task storage.

Native close invokes `closed`: it verifies exact WindowId, clears it, and spawns
an app-owned task with a one-second timer. Its AsyncApp is weak; no window or
extension callback survives the delay. `check` requires no native windows and
strict next-cycle order. It consumes the stopped presentation session after the
existing settlement delay, retaining only its validated value record before
checking entities or Metal. Failure also drops any remaining session. Early cycles are warmup; final warmup captures
`app.leak_detector_snapshot`; later cycles call `assert_no_new_leaks` against it.
That snapshot records IDs, not strong entity handles. The assertion ignores
entities already present in the baseline and checks newly created live handles.
See [GPUI definition](../../../../vendor/gpui/src/app.rs).

`record` writes/flushed bounded-cycle stdout records: checkpoint phase, optional
Metal and presentation-retirement checkpoints, and final complete. `sdk::contain` is deliberately inside the
global update lease, so an assertion panic returns an error without corrupting
borrowed global storage. Failure marks the audit and removes its subscription,
then writes failed best-effort. Missing/failed records cannot pass: the external
collector withholds continuation. Final success removes the subscription; remaining
global/task/device storage belongs to application shutdown.

The event trace is native window close → delayed app-owned check → process record
→ Python collector validation → stdin continuation → next OCaml window.
There is no delivery through a dead extension instance, and no native layout
callback synchronously enters OCaml.

From the repository root using [development setup](../../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path examples/resource_audit/rust/Cargo.toml --locked --lib
GPUIO_JOBS=2 ./scripts/gpuio exec dune build --profile release examples/resource_audit/main.exe
```

This separate Rust manifest/lock is outside the root workspace. Tests validate
bytes and deliberately retain a GPUI test entity: its check must return Panicked;
release/settlement must let the same baseline pass. They do not create real audit
windows or sample Metal. Run actual qualification through [main's collector](../../main.md).

The claim is no **new live GPUI entity handles** relative to a closed warmup
baseline. It is not zero entities, complete Arc/Objective-C ownership, process
RSS, GPU memory, or physical presentation. Instrumented timing is not ordinary
responsiveness evidence. Keep platform/full-cycle evidence separate from smoke
and compilation, and never suppress failed records to make resource checks pass.

The [presentation module walkthrough](presentation.md) explains immediate mount,
stop/close/drop ownership and why settled outcomes are not timing acceptance.
