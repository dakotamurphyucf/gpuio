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
integration status. Remaining acceptance: implement Wayland export ownership,
expose public capabilities and run the Linux build/unit gate.
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
These results do not exercise a Linux window or a real portal service. Wayland
parenting remains Unsupported until its export adapter is implemented.
