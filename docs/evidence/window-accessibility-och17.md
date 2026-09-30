# Initial macOS window accessibility focus

Local checkpoint: 2026-09-30, macOS 14.5 arm64. This covers external AX focus
getters and actual OS keyboard routing, not VoiceOver speech or notification
delivery. OCH-17 and the macOS release audit remain open.

## Failure and ownership

Opening a second Settings gallery window while the application was already active
allowed normal keyboard editing but left its editor's `AXFocused` false, including
after ten seconds of polling. The first window reported focus correctly.

The pinned GPUI macOS backend installs its AccessKit adapter after creating the
native window. A newly opened window can already be key at this point, so its
first key-window notification arrived before the adapter existed. AccessKit
0.26.3's `SubclassingAdapter::for_window` initialized host focus to false and
waited for a subsequent key-window change. AccessKit therefore suppressed the
focused accessibility node despite correct GPUI editor focus and AppKit input.

The vendored adapter now initializes this existing state from
`NSWindow.isKeyWindow`. The generic view constructor keeps its documented
before-first-focus behavior. Later key-window updates continue through the
existing GPUI callback, which uses the same native property. There is no synthetic
activation, timer, new editor state or protocol change. The source version,
licenses, reconstruction patch and checksums are recorded in
[adapter provenance](../../vendor/accesskit-macos/GPUIO.md).

## Regression

The Settings two-window public-API fixture now requires:

- The new second window's clicked editor reports `AXFocused=true` and receives
  an actual Command+A / character edit while the first window's value is unchanged.
- Switching to the first window reports its editor focused and the second's
  editor unfocused, with independent actual keyboard input.
- Switching back to the second window restores its focus report and accepts
  another edit, while the inactive first editor reports unfocused.
- Resetting the first value preserves the second value; closing the second
  leaves the first editable and correctly reporting focus.

The regression fails before the patch at the initial second-window assertion and
passes after it. It uses bounded polling for asynchronous paint/AX publication,
checks the actual window at the mouse target, and closes the windows afterward.
The focused run used a 90-second process-group watchdog and exited zero.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section settings-windows
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_editor --no-run
# Run the Cargo-reported native_editor executable with a bounded watchdog.
```

The native editor regression also exits zero under a 90-second process-group
watchdog: grapheme editing, clipboard, Tab policy, revision guards, resizing,
AppKit accessibility focus/value actions and native `NSTextInputClient`
marked/committed text calls pass. Those direct text-client calls are separate
from real OS input-method candidate-window acceptance.

A freshly staged installed-library consumer with its independently locked native
backend passes the same two-window OS/AX regression and closes normally under a
90-second process-group watchdog. It uses the composed backend's existing
AccessKit path patch, without installing into the opam switch. The source patch
also reconstructs byte-for-byte from the recorded upstream file checksum.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-window-focus-20260930-1
python3 scripts/test_gallery.py --section settings-windows --executable /private/tmp/gpuio-window-focus-20260930-1/consumer/_build/default/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
```

The full Dune build/expect/format suite, Python syntax check, source rustfmt,
diff checks, structural catalog audit and native all-target Clippy pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --features native-image-tests --all-targets -- -D warnings
```

The temporary consumer path is
evidence only, not a build input or clean-machine distribution claim. The complete
Settings walkthrough passed at the preceding warm-row checkpoint; this adapter
checkpoint reruns the focused window regression and native editor coverage.

Disabled Settings row discoverability is a separate open issue; the panel's
current Inert subtree policy still hides disabled fields from AX. These checks
also do not establish Linux GUI acceptance, full multi-window accessibility
semantics or clean-machine distribution.
