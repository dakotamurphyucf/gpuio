# Color Studio

Build with `GPUIO_JOBS=2 ./scripts/gpuio build examples/color_input/main.exe`, then
run `_build/default/examples/color_input/main.exe`.

This public Bonsai/Eio example demonstrates one native color editor: hex entry,
HSLA channels, draggable rails, palette swatches and clear. Rust owns drafts,
composition, history and preview values. Bonsai receives observations and sends
explicit commands; it does not echo observations back into editor text.

Enter commits a valid draft; Escape cancels. Tab finishes the old field and moves
through the control. Arrow keys on channel rails adjust values; Page Up/Down use
larger steps. Programmatic Set and Reset validate first, cancel any active edit,
replace native fields and clear obsolete history. They emit observations, not
user commits. Opaque-only policy retains historical alpha values but rejects new
incompatible values. Reset restores the original mount seed.

Run `_build/default/examples/color_input/main.exe --self-test` for the real
OCaml/Rust command bridge, revision/lease guards, policy transitions, 64-request
admission limit, remount and close ordering. The test opens and closes a native
window. It does not substitute for external OS keyboard, accessibility, popup or
full appearance acceptance; the native and popup suites provide separate coverage; see the OCH-36 evidence ledger.

## Popup picker

Build `examples/color_input/picker.exe` and run
`_build/default/examples/color_input/picker.exe`. `Gpuio_eio.Color_picker` composes
the native color editor with the existing popover, focus restoration and dismissal
adapters. The application owns a confirmed value; channel previews and native
text commits only update the popup draft. Apply re-reads the native snapshot and
checks current policy/session before calling `on_change`. Cancel, Escape and
outside dismissal preserve the confirmed value. In a popup, Escape dismisses the
whole popup; the inline editor uses Escape to cancel its current interaction.

The example includes explicit external reset, read-only/disabled/opaque policies,
right-edge placement and use inside a dialog. For a historical value disallowed
by new policy, opening starts Empty when permitted, otherwise the original RGB
with opaque alpha (or opaque black for a disallowed empty value). The application
value stays unchanged until a successful Apply.

`_build/default/examples/color_input/picker.exe --self-test` covers Apply/Cancel,
in-flight confirmation cancellation, native remount, component deactivation,
policy changes, historical fallback, external reset and window closure.
`python3 scripts/test_color_picker.py` additionally exercises real macOS AX
fields/sliders and OS Enter/Escape/arrows, outside pointer dismissal, focus
restoration, nested dialog and right-edge placement. It targets and reaps its own
child app. Both pass locally. Native appearance/density/lifetime/workload acceptance also
passes; consolidated hosted macOS/Linux checks and merge remain pending. See
[the evidence ledger](../../docs/evidence/color-inputs-och36.md).
