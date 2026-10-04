# Semantic input hints — implementation contract

OCH-41 continuation, 2026-10-01. This is an implementation contract, not a claim
that the public hint API or autofill integration is finished.

Expose the pinned input content-type vocabulary as a typed `Text_input.Content_hint`
value, separate from password display and edit formatting. A config change must
retain the editor, text revision, selection, composition and history. Keep legacy
EditorConfig bytes stable and add a paired optional hint operation. Picker queries
and comboboxes default to no hint until an explicit contract includes them.

Password/NewPassword hints must require explicit password privacy; do not silently
turn a plain input into a password or expose its value through accessibility.
Other single-line hints project the pinned phone/email/URL/date/date-time roles;
multiline remains a multiline text input. The hint is metadata, not a validator,
keyboard layout, password generator or guarantee of OS/password-manager autofill.

A native window coordinator selects only the current, editable, eligible focused
editor. It clears its owned native hint on focus to another owner/non-input, disabling,
read-only, hiding/removal and window release. Unfocused editors cannot overwrite
another field's hint. Updates and cleanup must not touch editor content or history.
Provide an asynchronous exact-lease status query distinguishing inactive, exposed
native metadata and unavailable backend/mapping. “Exposed” must not mean autofill
was offered, accepted or supported for every OS/version/application configuration.
The wire/API status contract is specified below.

The pinned macOS helper installs contentType/setContentType: and keeps NSStrings
in a thread-local map keyed by NSView address. Copying its render-only sync would
leave cleanup and address-reuse obligations. The GPUIO adapter instead attaches
bounded retained hint/owner metadata to the native view through Objective-C
associated objects. Association ownership ends with the view; explicit clear and
a lease destructor release earlier. An older lease must not clear a newer one,
and a foreign setter must invalidate the previous lease's ownership.

Do not replace foreign implementations of contentType/setContentType:. If the
runtime protocol or compatible methods are unavailable, report unavailable.
Keep the native bridge within GPUIO; no GPUI/base fork change is needed for this
adapter. Native tests should cover actual AppKit getters/setters, owner handoff,
foreign property replacement, cleanup, independent views and method collisions
without claiming physical input/autofill acceptance.

Linux keeps the semantic accessibility metadata but reports native autofill-hint
exposure unavailable until a real backend is provided. CellularEid/CellularImei
have no pinned macOS string mapping and must report unavailable, not success.
Real macOS focus/IME/AX/autofill checks and final Linux builds remain required.

## Status API and request lifecycle

Keep ordinary editing commands snapshot-returning. Add a dedicated
`Gpuio_eio.Text_input.content_hint_status` effect using the same exact observed
window/node lease and the shared 64-request editor budget. Its result uses the
existing editor command errors and a typed content-hint status:

```ocaml
module Status : sig
  module Unavailability : sig
    type t = Backend | Mapping | Native_view
  end
  type hint = Input_content_hint.t
  type t =
    | Inactive of hint option
    | Exposed of hint
    | Unavailable of hint * Unavailability.t
end
```

`Inactive` covers absent config or a field that is not the eligible editable
focused owner. `Exposed` means this lease owns the current native-view property;
it does not promise an OS/provider recognizes or acts on it. `Mapping` means the
selected backend has no mapping for this value, `Backend` means the platform/runtime
has no hint facility, and `Native_view` means the view cannot accept the owned
adapter (including conflicting native methods). On Linux a configured eligible
field reports Backend unavailable. EID/IMEI report Mapping unavailable on macOS.

Use a private wire editor command `ReadContentHintStatus` (append command tag 7)
and a distinct `ContentHintStatus` editor response (append result tag 2). Keep the
public `Text_input.Command` snapshot contract unchanged. These paired additions
belong to unpublished protocol epoch 3; rebuild both runtimes together. The host resolves the
current focused owner and checks native ownership at query execution; it must not
emit a text observation merely to answer metadata. Return the configured hint in
inactive/unavailable results so callers do not accidentally describe an older
config as current after an asynchronous response.

Generalize the private App editor-request completion boundary to carry wire editor
results, with typed projections for ordinary command snapshots and hint status.
Preserve its current correlation/window/node checks, shared capacity, close and
exception-safe cleanup. A response of the wrong kind fails that request with
Native_failure. Replies for another lease/correlation must not consume a request;
duplicates are ignored. The hint query must not overwrite controller observations
or expose text through its response. Add delayed-reply, close, shared-capacity and
wrong-result-kind expectations before treating this API as complete.
