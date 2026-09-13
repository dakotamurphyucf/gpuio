# OCH-11 local menu adapter evidence

Platform: macOS arm64, stock OCaml 5.3.0, Bonsai/Core v0.17, Eio,
Dune 3.24.2, Rust 1.97.1. GPUI remains pinned to
`a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`. No upstream/vendor patch was added
for this menu adapter. This is local component evidence, not completion of OCH-11
or hosted/Linux acceptance.

## Reproduction

From the repository root, run these sequentially because Dune also invokes Cargo:

```sh
./scripts/gpuio exec dune build @runtest @all @fmt
./scripts/gpuio exec dune exec examples/menus/main.exe -- --self-test
./scripts/gpuio exec cargo test --workspace --locked
./scripts/gpuio exec cargo clippy --workspace --all-targets --features native-tests --locked -- -D warnings
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_menus --locked
./scripts/gpuio exec cargo test -p gpuio-native --features native-tests --test native_controls --locked
```

The last two commands open actual windows and exercise native input. The menu
scenario is included in `native_controls`; `native_menus` runs it independently
for faster local diagnosis. The combined suite passed after the final native menu
behavior changes. An earlier run failed in the existing tooltip hover scenario;
that failure did not recur in the final combined run. No tooltip-specific fix or
proven diagnosis of that earlier failure is claimed.

## What the native checks prove

- Disabled/separator skipping, nested arrow navigation, activation and Escape.
- macOS accessibility menu-item role, checked/disabled values and real AX press.
- macOS `NSMenu.performActionForItemAtIndex` dispatch into the shared command
  registry; stale installed actions rejected after disable/re-enable.
- Actual right mouse events over the native editor, selection preservation, Copy
  without an OCaml editing round trip, and restoration of editor focus.
- A 1000-command menu uses bounded visible row anchors. Wheel input moves its
  native scroll offset; End reveals and activates the last command.
- Cascading menu panels extending beyond a containing popover remain inside its
  hit region. Escape dismisses the menu before the containing popover.
- Hiding a dropdown with Visibility or Display closes detached panels and releases
  focus; Enter cannot invoke the hidden menu. Disabling an open trigger also
  closes it and releases focus.
- Native macOS menus follow focused command scopes, reject hidden/stale sources,
  preserve the active window's definitions when an inactive window is created,
  switch on activation, and restore the surviving window's menu after close.
- Removing the fixtures releases menu state and installed platform definitions.

The combined suite also passed radio/select/combobox, focus scopes,
dialog/popover, tooltip, command/shortcut, native editor-target and accessibility
checks after changing submenu hit-region storage and overlay priority spacing.
The public Bonsai/Eio example separately passed menu mounting, native render
acknowledgements, registry changes, nested-scope removal and shutdown.

## Findings incorporated into the implementation

App menus must refresh ownership when window activation is delivered, even when
the surviving window has no new OCaml tree data. Waiting only for a subsequent
view render left the closed window's definitions installed in the native test.

Hiding a trigger must explicitly close its retained, detached popup surfaces and
release native focus; simply applying hidden styling to the trigger is inadequate.
Menu opening and action delivery also recheck visibility.

The right-click test initially borrowed the receiving View while injecting the
mouse event. The GPUI backtrace identified a test-harness reentrant borrow. Event
injection now uses the window-level context, matching the other native input
helpers; no product workaround was necessary for Copy.

## Coverage limits

The native tests use programmatic input through actual GPUI/AppKit windows; they
are not a physical keyboard/IME candidate-panel or complete screen-reader audit.
Linux code paths include an in-window platform-bar keyboard check, but this local
macOS run cannot establish Linux GUI behavior. Required Linux builds/tests and
consolidated hosted CI remain part of final OCH-11 delivery; full graphical Linux
acceptance is tracked in OCH-17. Native menu accelerator labels/OS key equivalents
are not exposed yet; executable shortcuts use the shared command matcher.

See [the menu contract](../design/native-controls.md#menus) for the public API,
limits, styling and command ownership semantics. Remaining OCH-11 families and
OCH-12 animations are not implied by this component report.
