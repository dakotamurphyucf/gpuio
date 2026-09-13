# OCH-11 file-dialog bridge and macOS adapter evidence

Local macOS arm64, 2026-09-13. Stock OCaml 5.3, Bonsai/Core 0.17, Dune 3.24.2,
Rust 1.97.1 and GPUI `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
This is a local bridge/macOS checkpoint, not completion of file dialogs or
OCH-11. No hosted or Linux GUI acceptance is claimed.

## Checks

```sh
./scripts/gpuio exec cargo test -p gpuio-native --locked --features native-tests --test native_file_dialog
./scripts/gpuio exec cargo clippy --workspace --locked --all-targets --features native-tests -- -D warnings
./scripts/gpuio exec cargo test --workspace --locked
./scripts/gpuio exec dune build @runtest @all @fmt
```

The final native file/directory/save suite passed (66124). Full workspace Clippy,
Rust tests and Dune build/expect tests/format passed afterward (39236), including
production static-library linking with the direct macOS panel dependencies.

The native suite creates a real GPUI window and AppKit sheets. It verifies:

- Visible sheet attachment and mixed/multiple configuration.
- Busy when a sheet already belongs to that parent.
- The native Cancel action, repeated owner cancellation and owner drop while
  presentation is starting. Cancellation hides the actual panel; a dropped
  outstanding owner returns Closed exactly once.
- Weak completion-block ownership: the owner state is no longer retained after
  disposal, including cancellation before presentation has settled.
- Real native selection of the repository LICENSE file and the `/tmp` directory.
  The assertions use canonicalization only to compare file identity; production
  results preserve the native URL's exact bytes.
- Acceptance of a native save destination ending in `.sql.s`, with exact returned
  filename and URL path and no created destination file.

## Native test findings

Calling `NSSavePanel.ok:` directly raised an Objective-C exception on this machine:
the method reports that it is not implemented. LLDB located the throw in that
test call. Cancellation and presentation had passed before the exception.
Constructed NSEvents, application/window routing, targeted CG keyboard events and
in-process accessibility traversal did not activate the remotely hosted save
button. A captured native-window image showed a normal enabled button.

The test now uses the public AXUIElement API on a worker thread targeting its
own process. This leaves the main thread available to answer AppKit accessibility
requests. It selects the file-browser row through AXSelected and activates the
native buttons through AXPress. Accessibility access is available on the local
machine; the test emits a specific diagnostic and fails if it is unavailable.
Hosted accessibility permission/GUI execution remains unverified. The harness
explicitly reports skipped AppKit coverage on non-macOS platforms.

The AX helper's initial null-value ownership bug and a later test result-borrow
lifetime error were fixed before the final run. Those were test harness issues,
not claimed fixes to GPUI or AppKit. Failure screenshots target only the native
picker window and use a process-specific temporary filename.

## Correlated bridge and public effects

The bridge now has independent OCaml/Rust request and result fixtures, bounded
raw-path decoding and mailbox batch-size tests. The latter encodes large selected
responses to verify every drained batch remains within the 1 MiB envelope and
that queued responses retain the window slot until drained. Forged path counts
and lengths are rejected before allocating their declared payloads.

The native request-manager suite passes selection disposal, Busy, exact window
generation, cancellation of simultaneous requests on two windows, and absence
of duplicate/late responses after cancellation. The public Bonsai/Eio lifecycle
example passes Not_ready before opening, Busy on overlap, Closed after window
closure and Closed on application shutdown.

```sh
./scripts/gpuio exec dune exec examples/file_dialogs/main.exe -- --self-test
./scripts/gpuio exec dune build examples/file_dialogs/main.exe
./scripts/gpuio exec cargo test -p gpuio-native --locked --features native-tests --test native_file_dialog --no-run
# Use the exact executable path printed by Cargo (its hash depends on the build):
python3 scripts/test_file_dialog_read.py --driver target/debug/deps/native_file_dialog-<hash>
```

The last command passes actual file selection through the native bridge into a
Bonsai effect and reads the selected LICENSE file with Eio. It targets only the
child PID it creates and terminates that child on timeout/failure. The public
example also verifies the exact selected path and Apache license content. The
native driver uses AXSelected and AXPress; no synthetic success is injected.
The save example intentionally does not write a file.

During bridge validation the original AX traversal repeatedly failed to select a
directory, sometimes observing cancellation. Diagnostics showed an enabled button
and a traversal through unrelated file-table rows before reaching it. The helper
now excludes file tables when looking for action buttons and searches children
from the end, where the sheet actions occur. The native suite subsequently passed
without changing production AppKit ownership. This establishes a working targeted
test; it does not establish the cause of every earlier cancellation.

## Latest bridge checkpoint validation

All of these local macOS checks passed after the bridge and native test changes:

| Check | Observed coverage |
| --- | --- |
| Native `native_file_dialog` harness (29279) | File, directory, two-file selection, exact save path, panel disposal and correlated request ownership |
| Workspace Clippy / Rust tests (24510) | All targets including native-test code compile cleanly with `-D warnings`; unit/protocol/mailbox tests pass |
| Dune `@runtest @all @fmt` (95723) | Full OCaml build, expect tests, formatting and static linking |
| Public `--self-test` (95723) | Pre-open/overlap errors, native window-close and application-shutdown cancellation |
| Public AX selection/read script (95723) | Real selected path delivered through Bonsai and read with Eio |

Two-file selection uses AXSelectedRows to extend the browser selection. Setting
AXSelected on the second row replaced the first selection on this AppKit version;
the test now selects both rows explicitly and verifies both returned paths.
Native presentation settling and bounded AX readiness retries precede actions.
These are test-driver adaptations, not changes to application selection policy.

## Still required

Capability reporting is now implemented as described below. The Linux portal
protocol and native worker ownership have local unit evidence; the Wayland export adapter compiles
locally but its system-libwayland tests await Linux execution. See the
[portal report](linux-file-portal-och11.md). Linux build/unit checks and
consolidated hosted CI/merge remain required. No Linux GUI acceptance is claimed.

## Public capabilities checkpoint

`Gpuio_eio.File_dialog.capabilities window` now returns a typed snapshot without
presenting a picker. Core exposes selection/cardinality and save predicates.
Independent OCaml/Rust fixtures verify the appended query/result tags and all
selection-support variants. OCaml rejects malformed enum/Boolean tags, truncation
and a payload of the wrong result kind. The bridge advertises bit 524288
(total mask 1048575); actual backend availability remains a runtime query.

Local macOS validation passed:

| Check | Evidence |
| --- | --- |
| Workspace all-target/native-tests Clippy, Rust tests, Dune `@runtest @all @fmt` (10774) | New protocol fixtures, typed Core/Eio API, existing unit/expect tests and full linking |
| Native file-dialog harness (9724) | Capability probe returns all supported AppKit modes with no attached sheet or retained panel; existing selection/save/ownership assertions pass |
| Public `--capabilities-self-test` (87486) | Actual native capabilities, pre-open Not_ready, overlap Busy, pending window-close and app-shutdown Closed |
| Public `--self-test` and AX read script (87486) | Existing picker cancellation and real selection through Bonsai into an Eio LICENSE read still pass |

The native driver for this run was
`target/debug/deps/native_file_dialog-8d8e8113a91a4994`. Reproduce the public checks:

```sh
./scripts/gpuio exec dune exec examples/file_dialogs/main.exe -- --capabilities-self-test
./scripts/gpuio exec dune exec examples/file_dialogs/main.exe -- --self-test
python3 scripts/test_file_dialog_read.py --driver target/debug/deps/native_file_dialog-<actual-hash>
```

An initial capability self-test timed out before it had stage diagnostics. A
rerun with diagnostics passed without changing the runtime. The final test
queries until the native window is open instead of waiting for a painted frame:
capability queries do not require painting, and frame waits unnecessarily couple
this test to desktop occlusion/activation. The original timeout's exact stage
was not established. Stage-specific timeout errors remain in the test.

These checks ran locally with real macOS windows. Linux GUI acceptance and the
three system-libwayland protocol tests remain unverified on this machine; the
latter are explicitly ignored on macOS and enabled for the Linux build/test gate.
No hosted CI or merge is claimed for this checkpoint; OCH-11 remains In Progress.
