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
full appearance acceptance; those remain tracked in OCH-36.
