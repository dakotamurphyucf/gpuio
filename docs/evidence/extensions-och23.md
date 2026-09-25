# OCH-23 local acceptance checkpoint

Status: implemented and validated locally on macOS arm64, 2026-09-24. Required
hosted macOS/Linux checks have not run for milestone 05 yet. This is not a claim
of Linux graphical acceptance or a completed milestone.

## Contracts and implementation

See [the extension contract](../design/extensions.md), public
`Gpuio.Extension`/`View.extension` interfaces and `gpuio-extension-sdk`.
The native host admits an immutable catalog, validates package payloads before
publishing tree changes, mounts retained native instances and queues typed events.
Commands execute once when transactions apply, including multiple transactions
between frames. Reset, property changes, hiding, disabling, removal and window
close have explicit lease behavior. Rendering does not synchronously enter OCaml.

`scripts/compose_backend.py` generates one Cargo static archive and a Dune virtual
library implementation. The independent sample imports only the public SDK and
exposes typed OCaml properties, commands and events. Default-backend applications
continue to build. `scripts/test_extension_consumer.py` stages public installed
libraries under its own prefix, copies the component into another Dune project,
builds a consumer and optionally runs its real native smoke scenario. It does not
install into an opam switch.

## Local checks

All commands use the repository environment, stock OCaml 5.3/Bonsai v0.17 and the
pinned Rust/GPUI toolchain. `GPUIO_JOBS=2` bounds compiler concurrency.

| Check | Local result |
| --- | --- |
| `./scripts/gpuio build` | Pass, all existing examples plus composed extension consumer |
| `./scripts/gpuio test` | Pass, OCaml expect suites and Rust workspace tests |
| `./scripts/gpuio check-fmt` | Pass |
| Cargo Clippy for native, SDK and example crate, all targets with native-tests, `-D warnings` | Pass |
| `cargo test -p gpuio-native --test extension_catalog` | Pass: exact catalog matching, frozen registration, bounded/schema/domain/panic rejection before atomic publication |
| Paired protocol fixtures | Pass: independent OCaml/Rust requests and events, arbitrary binary bytes, truncated request rejection |
| Extension reconciliation | Pass: handler/generation fencing, disabled data rejection, command observations and schema replacement |
| Native extension test | Pass: real window, Tab/Space, macOS AX button activation, pointer-disabled keyboard, hidden/disabled sinks, updates, command execution without intervening frames, reset, contained command panic and recovery |
| Native repeated cleanup | Pass: repeated mount/unmount restores retained-byte baseline; all mounted instances unmount; window close rejects late events |
| Original composed consumer `--smoke` | Pass: catalog check, typed command acknowledgment, correlated paint and automatic shutdown |
| Independent consumer script with `--run` | Pass: separately staged public packages and copied component, native build/run/shutdown; relocated Cargo.lock matches the checked-in consumer lock |

The intentional package panic in the native test prints Rust's normal panic hook
message, then is contained and observed as `Failed Panicked`; the test exits
successfully. The accessibility check initially found duplicate focus registration
on wrapper and child. The host now records the component's primary handle for
traversal while only the component registers its accessible focus element.

## Remaining validation and limits

CI definitions include independent-consumer compilation on both platforms and
native extension/consumer checks on macOS. Record the eventual checked commit,
run and merge in Linear. Linux GUI acceptance remains the OCH-17 release gate.

This SDK accepts trusted statically compiled component code. Hook and guarded
input panics are contained; arbitrary custom GPUI element internals, unguarded
callbacks, process aborts and unsafe-code corruption are not sandboxed. Native
authors own nested codec bounds, internal focus policy and cancellation of their
resources. Dynamic loading and a stable binary ABI are outside this contract.
