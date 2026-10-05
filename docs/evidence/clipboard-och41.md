# Application clipboard writes — OCH-41

Local macOS 14.5 arm64 implementation and validation after `d74eb31`, on the
milestone-07 branch. This closes the standalone application-text operation gap
identified by the exact pinned Clipboard review; it does not certify the whole
catalog, Linux GUI behavior or release distribution.

## Interface and ownership

See [the contract](../design/clipboard.md) and `lib/eio/clipboard.mli`.
`Gpuio.Clipboard.Text` validates UTF-8, absence of NUL and the 256 KiB limit.
`Gpuio_eio.Clipboard.write_text` owns its serialized payload and uses the existing
bounded, correlated desktop request lane. Rust invokes GPUI on the native UI
thread without a synchronous OCaml callback. Success reports that invocation,
not durable clipboard ownership or delivery to another application. No desktop
identity or window is required; shutdown resolves pending requests as Closed.

The paired desktop request tag is 8, appended within unpublished exact-source
bridge epoch3. No existing tag, command limit or queue budget changes.
The Bonsai `Clipboard.Copy` composition captures the current value at activation,
handles typed failures, suppresses duplicates while busy/copied, invalidates
obsolete feedback on value/disabled/deactivation changes, and expires successful
feedback through the shared Bonsai clock after two seconds. An admitted write
cannot be undone by hiding the button. The default view uses ordinary native
button focus/keyboard/accessibility; custom icon/tooltip compositions use its
public state and action with existing views.

## Completed local validation

`GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 @runtest
examples/gallery/main.exe examples/color_input/picker.exe
examples/navigation/main.exe` passes in `clipboard-build-002.log`.
The first attempt failed OCaml record-label inference in the new controller;
explicit input type annotations resolved it without changing the design.
The suite includes independent clipboard request bytes, Core payload rejection,
and three controller scenarios covering delayed acknowledgement, duplicate
suppression, exact clock expiry, value/disabled/branch retirement, obsolete
completion after a newer request, explicit retry and disabled admission.

`python3 scripts/test_gallery.py --section clipboard` passes in
`public-ci-clipboard-reruns-001.log`. A real public gallery window performs:

- AX button activation copying literal Unicode/newline text, verified from
  NSPasteboard; Copied appears and returns to Copy after the shared-clock delay.
- Native Space activation of the focused current-value button, followed by
  incrementing application state and copying the new value.
- Accepted page retirement/reentry with idle feedback, then another literal copy.
- Normal window close and successful process exit.

`scripts/mac_clipboard.py` saves every eagerly readable item/type representation
in memory before the sequence, then restores and compares those bytes in a
`finally` block. It fails before mutation if a representation is unavailable or
exceeds the bounded preservation budget. Original clipboard payloads are never
logged. The actual run passed restoration; two portable control-flow tests also
pass for success, exceptions/interrupts and unavailable initial data. This is
not a claim about surviving forced termination or preserving lazy providers.

Broader Rust/hosted/independent-consumer qualification is recorded separately as
it completes. These checks do not qualify Wayland/X11 clipboard behavior; that
remains OCH-47. Other release and catalog items remain open.

Logs are under ignored `scratch/agents/root-20261004-resumed/`; no scratch path
is a build dependency. The versioned source/tests reproduce the scenarios.

The full protocol suite passes **394 tests**. The combined-feature native suite
passes **923 tests with two existing macOS private-bus skips**; strict all-target
native/protocol Clippy and full Dune expect/build/format checks also pass
(`clipboard-final-validation-001.log` for protocol, `-003.log` for the final
native/lint/Dune checks). The new native clipboard fixture exercises the reserved
response lane, correlation without identity/windows, preserved prior clipboard
on invalid input, and empty replacement. Its first attempts incorrectly bypassed
response reservation and expected the `ClipboardItem.text` helper to return
`Some("")`; those fixture mistakes were corrected to use mailbox admission and
compare the actual empty clipboard item. Production admission guards remain.
