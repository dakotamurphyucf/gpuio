# A separately packaged native counter

This example contains an ordinary OCaml library in `ocaml/` and an independently
consumable Rust crate in `rust/`. The Rust implementation imports only the public
`gpuio-extension-sdk` and its pinned GPUI re-export. `schema.txt` is the exact
schema text whose SHA-256 appears in both implementations.

The component displays a counter with pointer, Space/Enter and accessibility
activation. Properties set its value and step; a sequenced command sets its value;
native activation sends a typed value observation. Values are 0–100, steps 1–10.
Payloads have fixed lengths and are validated before indexing. Each callback uses
the revocable SDK event sink. Pointer disabling leaves keyboard and accessibility
activation available; hidden/disabled/obsolete instances reject callbacks.
The accessible button reports disabled state from its current event lease. The
host group alone does not propagate that metadata to controls inside the package.

The crate declares SDK version `=0.0.0`, patched to the selected GPUIO checkout by
the application composer. These are experimental source packages, not published
registry packages or a stable binary plugin ABI. Consumers pin the complete
source checkout and commit their Cargo.lock.

See [the consumer](../extension_consumer/README.md) and
[the SDK contract](../../docs/design/extensions.md). Component authors write Rust;
application consumers use the typed OCaml library and a composition manifest.

For local lifetime checks, `GPUIO_COUNTER_TRACE=1` enables this package's bounded
per-lifecycle diagnostics: mount, unmount, component drop, last callback-value
drop, and accepted command values. Normal runs are silent. The reference-counted
value trace detects callbacks retaining data after the component object drops;
it is not a new SDK API or event-schema contract. Signal Studio's workload
verifies these traces across generation replacement and window close/reopen.
