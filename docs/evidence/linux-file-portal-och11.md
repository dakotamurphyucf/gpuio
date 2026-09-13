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

The following native-integration checkpoint supersedes the original protocol-only
integration status. Remaining acceptance: expose public capabilities and run the
Linux build/unit gate, including the Wayland ownership tests below.
Actual portal GUI validation belongs to the deferred Linux GUI gate (OCH-17).
These checks do not complete file dialogs, OCH-11 or milestone 2.

See the [design and integration requirements](../design/linux-file-portal.md).

## Native integration checkpoint

The native X11 manager now calls the portal worker with an exact scalar parent
ID. Its platform-independent ownership code is also compiled in macOS unit tests;
the production macOS library continues to use AppKit. No dependency versions changed.

The native library's 15 tests pass, including seven portal-adapter tests covering
concurrent cancellation, broadcast completion, wrong window generations, Busy,
abandoned/panicked workers, manager clone/drop ownership, and cleanup failures.
A concurrency test blocks transport delivery after the worker has claimed its
response: close retains an incomplete barrier until delivery is released, and
the unconditional quit drain waits for completion. It uses an actual worker
thread and wake-pipe transport, not a fabricated completion flag.

Additional regression checks passed on macOS arm64:

| Command/check | Coverage |
| --- | --- |
| `cargo test -p gpuio-native --features native-tests --test native_file_dialog --locked` through `scripts/gpuio exec` (14749) | Real AppKit file/directory/multiple selection, save path, cancellation and request ownership |
| Workspace Clippy, Rust tests and Dune `@runtest @all @fmt` (67827) | Native adapter compilation, existing tests, static linking and formatting |
| Public file-dialog `--self-test` and `scripts/test_file_dialog_read.py` (27003) | Real window/shutdown cancellation and native selection through Bonsai into an Eio LICENSE read |

The native driver's executable was
`target/debug/deps/native_file_dialog-ef02126aadd51016` for this run. Cargo hashes
are build-specific; obtain the path from Cargo when rerunning the read script.
These results do not exercise a Linux window or a real portal service. The next
checkpoint below adds the Wayland adapter and records its outstanding validation.

## Wayland adapter checkpoint

The new `gpuio-wayland` workspace crate and native integration compile on local
macOS arm64. It reuses the existing locked wayland-client 0.31.15,
wayland-backend 0.3.17 and wayland-protocols 0.32.13, with system-client/dlopen
features consistent with pinned GPUI. No existing dependency version changed.
The production macOS native library still uses AppKit; the guest adapter is
compiled on macOS through its own workspace crate and native dev dependency.

The native X11-only feature configuration also passes local Clippy with warnings
denied (13197): `cargo clippy -p gpuio-native --lib --tests --no-default-features
--features x11 --locked -- -D warnings` through `scripts/gpuio exec`. This checks
conditional Rust compilation on macOS; it is not the required Linux build gate.

Full workspace Clippy with all targets/native-tests and warnings denied,
workspace Rust tests, and Dune `@runtest @all @fmt` pass locally (19357).
The existing native library's 15 unit tests pass. **Three new guest-queue tests
are explicitly ignored on macOS**, because system libwayland-client is absent.
They compile; no runtime pass is claimed. The tests use a real libwayland client
against a private Wayland wire peer, with the host queue performing all reads:

- 64 exports across two distinct surfaces, v2 preference, exact surface IDs,
  one shared guest registry, matching export destructors and final exporter drop.
- v1 fallback, cancellation before registry dispatch and before Handle delivery,
  followed by successful reuse of the host connection.
- Missing export protocols and invalid handle results, with cleanup.

The Linux foundation job already installs libwayland-dev and runs workspace
tests; these tests are enabled there without a compositor, D-Bus daemon or GUI.
Their results must be checked in the consolidated CI pass. They do not validate
compositor toplevel policy or a real portal's parenting; OCH-17 retains that GUI
coverage. No CI was submitted for this local checkpoint.

Source review found that Wayland's server registry resource survives client proxy
destruction until disconnect (`wl_display.get_registry` in pinned wayland.xml).
The adapter therefore shares one registry per app display and removes its client
proxy after initial binding. This avoids per-dialog server allocations and idle
registry-event accumulation. Native shutdown drains export workers, then drops
the shared guest display before GPUI clears windows. All foreign native objects
remain subject to the explicit lifetime contract in the design document.
