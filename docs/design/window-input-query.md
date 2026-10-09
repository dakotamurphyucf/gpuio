# Window-wide focused-input query

OCH-41 implementation contract, extending the pinned WindowExt helper mapping.

`App.Window.focused_input` is an asynchronous metadata-only query on the exact
window generation. It returns the current eligible native text input, or None.
Kinds cover Input, Textarea, Combobox, OTP, NumberInput, color text fields and the
command palette query. Read-only inputs remain discoverable. Disabled, hidden,
unmounted, clipped or modal-blocked owners are excluded. Color sliders/buttons
and ordinary controls are not text inputs. Application deactivation alone does
not discard the window's retained keyboard focus.

`Window.Input.t` is an opaque observation with a kind and generation-checked
window/node identity. Composite color fields identify their owning Color_input
controller; its typed observations remain the source of field/draft details.
It contains no text, selection, composition, password or
OTP value and no Rust entity. Matching helpers compare a known controller
snapshot with that identity. Mutations continue through existing typed controller
APIs with their exact-lease/revision rules. Observing focus does not give a delayed
operation authority over a replacement mount, nor promise focus remains there.
A boolean has-focused-input is derived from the returned option; no second native
query is required.

Use the existing 64-request window lane, correlation/generation validation and
close cleanup. The internal window request/response variants grow additively;
public Window.Command remains a concrete command-only type with conversion at
the bridge boundary. A query response cannot satisfy a command request, or vice
versa. Unknown/malformed input kinds and invalid node identities fail decoding.
No per-frame focus observations, text snapshots or new event stream are added.

Window-wide text selection has a [separate bounded contract](window-selection.md).
It respects the active native selection scope and includes renderer-local
selections; clipboard reads are not a substitute. Focused-input metadata does
not provide access to editable input selection or values.
