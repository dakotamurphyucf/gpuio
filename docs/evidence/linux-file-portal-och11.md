# OCH-11 Linux portal protocol evidence

Local macOS arm64, 2026-09-13, Rust 1.97.1. This checkpoint implements the
`gpuio-portal` request layer, not a working Linux GPUI picker integration.

Passed commands:

```sh
./scripts/gpuio exec cargo test -p gpuio-portal --locked
./scripts/gpuio exec cargo clippy --workspace --locked --all-targets --features native-tests -- -D warnings
./scripts/gpuio exec cargo test --workspace --locked
./scripts/gpuio exec dune build @runtest @all @fmt
./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

Fourteen focused tests passed (39627); workspace Clippy, Rust tests and full Dune
checks passed together (61685). The tests exchange real D-Bus messages over private
Unix socket pairs with preauthenticated, named peers. They cover:

- Actual OpenFile options, byte-array folder hints and signal-before-method-reply
  ordering; exact alternate-handle matching and selected-connection disposal.
- Cancellation before handle creation, retry on the returned handle, owner drop
  without a Response signal, and no OpenFile for a pre-cancelled request.
- Explicit cleanup failure, method timeout, socket disconnect, foreign-namespace
  rejection and exact service-disappearance signal filtering.
- Capability discovery from the pinned service owner after activation, unavailable
  service handling, directory-version/mixed-mode policy and exact save hints.
- Raw non-UTF-8 paths, preservation of dot segments/extensions, all-or-error URI
  handling, selection cardinality and per-path/count/aggregate bounds.

No daemon, Linux VM, container, GUI window, toolchain switch or global package
installation was needed for these tests. The fixture does not validate SASL or
real session-bus discovery/activation. Cargo.lock adds only the new workspace
package and references existing locked dependencies; existing versions are unchanged.

Remaining acceptance: connect the worker to GPUI; implement exact X11/Wayland
parenting and safe export ownership; await cancellation on window/application
close; expose public capabilities; run the Linux build/unit gate. The native
Linux manager still returns Unsupported until that integration is implemented.
Actual portal GUI validation belongs to the deferred Linux GUI gate (OCH-17).
These checks do not complete file dialogs, OCH-11 or milestone 2.

See the [design and integration requirements](../design/linux-file-portal.md).
