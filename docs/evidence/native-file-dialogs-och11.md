# OCH-11 macOS file-panel adapter evidence

Local macOS arm64, 2026-09-13. Stock OCaml 5.3, Bonsai/Core 0.17, Dune 3.24.2,
Rust 1.97.1 and GPUI `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`.
This is a local native-adapter checkpoint, not completion of file dialogs or
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

## Still required

This adapter has no application request/result bridge yet. The Eio/Bonsai API,
correlation and bounded mailbox accounting, capability reporting, runtime
window-close/shutdown cancellation, late-result handling, public example and
Linux portal backend remain to be implemented and tested. Parent lifetime is
currently demonstrated by dropping the native panel owner; it is not evidence
that the GPUIO application runner already cancels a picker on window closure.
Actual multiple-file selection and result limits also need bridge/native
integration coverage; configuration inspection alone does not prove that path.
