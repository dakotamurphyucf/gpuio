# One native editor behind a segmented code display

[otp.ml](otp.ml) demonstrates six-digit verification and eight-character
case-sensitive alphanumeric recovery fields. A filled field is not authenticated;
this demo sends no request. The [README](README.md) gives exact `otp.exe`
build/run commands and separate macOS diagnostic drivers. [dune](dune) isolates
module `otp` and links the public Core/GPUIO/Bonsai/Eio APIs and PPX. No assets
or credentials are required. Setup and platform boundaries are in the
[development guide](../../docs/development.md) and
[platform policy](../../docs/platform-release-policy.md).

Read policies/value helpers, graph setup and final view before the large optional
self-test. `digits` validates length 6 with default Digits alphabet; `letters`
validates length 8 and `Ascii_alphanumeric`. `value policy text` canonicalizes
with `O.Value.of_string`, maps a typed input error to an `Or_error`, and treats
bad hard-coded seeds as programming errors. Seeds are `12` and `Ab`, not full
codes. Policy length is 1..32; full-width digits/Latin letters normalize to ASCII,
case is retained. Paste additionally strips ASCII whitespace and hyphens; it
does not strip arbitrary Unicode separators. Invalid/overlong input rejects
atomically rather than truncating. A value constructed under one policy must
still fit the mounted field's policy; its type alone does not encode compatibility.
See [Otp_input](../../lib/core/otp_input.mli).

`component` builds a persistent Bonsai computation graph. Application states
shown true, masked/disabled/read-only false and instruction status are allocated
once with `B.state`. `let%arr` reads their current ordinary values to derive
configuration/views and callbacks; `and` lists dependencies, not parallel work.
Setters create effects, whose construction does not execute a state update.
`B.return` makes the recovery config constant/reactive. `C.create` allocates
one native editor per code, with one `C.view` placement per controller. A third
unplaced controller is solely diagnostic `Not_mounted` coverage.

Rust owns accepted value, Unicode preedit draft, directional selection and undo
history. The cells are a native presentation of one editing session, not six
different application states. The accepted value is canonical ASCII; during
IME composition `Snapshot.draft` can contain separate Unicode preedit and offsets
are UTF-8 bytes into that draft. Otherwise ASCII byte offsets equal cell indices.
Policy is immutable within a placement: remount deliberately to change alphabet
or length. Changing config flags retains the native value. Seeds apply once per
mount; removing/remounting the six-digit view returns to seed `12`, not unsaved
native contents/history from its former lease.

`on_event` derives a callback from the status setter and returns `E.Many` effects.
In test mode one effect records events. Complete updates status to “Code filled.
No authentication request was sent”; Rejected displays typed input error;
Observed/Changed require no application status action. Complete follows a native
edit that changes accepted value to full length outside composition; programmatic
replacement, initial state and preedit never emit it. Command results must be
handled explicitly. `report` uses `E.Let_syntax`/`let%bind` to await a typed reply
then set revision/error status. `field`, `details` and `gap` render cells with
44-pixel height, length/policy count and Composing/Filled/Ready metadata. The
application never paints caret/cells or mirrors text through replacement.

For an interaction, finish the six-digit field with a native edit. Rust validates
and canonicalizes input, updates accepted value and emits Changed followed by
Complete as ordered boundaries. The controller observes asynchronously on the
OCaml UI domain; `on_event` updates status, Bonsai derives Filled metadata and
GPUIO submits a new native transaction. No authentication work runs. Fill sample
uses `replace` explicitly with selection End and undo Record; it updates the
native value but does not trigger Complete. Clear/Undo/Redo are also explicit
commands. Masking hides painted/accessibility text and disables Copy/Cut, while
snapshots still contain actual code: this is presentation privacy, not secure
storage. Read-only/disabled reject native editing, but explicit replacement and
clear remain permitted subject to composition and identity guards.

`App.run` keeps GPUI on the OS main thread and initializes one OCaml Eio UI domain
for graph/effects, mounting a 760×480 window. No file/network/verification producer
or application task cancellation occurs here. Close force-closes the window and
cancels window-scoped runtime/controller ownership. Use `request_close` for an
application close decision; read [App](../../lib/eio/app.mli) and
[the controller contract](../../lib/eio/otp_input.mli).

## Optional bridge diagnostic

`_build/default/examples/numeric/otp.exe --self-test` opens a window and prints
`GPUIO_OTP_PUBLIC_OK` on success. `B.map` derives observations;
`B.Edge.on_change` uses typed optional snapshot equality and a `started` ref to
run once. `latest`/`latest_alpha` are test-only controller references used after
mount changes. `expect`, `error` and `check` fail on unexpected results.
`frame` bridges native render callback completion via `E.Expert.of_fun`;
`settle` sequences two callbacks for native configuration/mount settling. These
callbacks and command acknowledgements are not physical frame/IME evidence.

The `concurrent` diagnostic helper starts effects within one UI turn using
`E.Expert.eval`, stores each indexed result and invokes its callback once all
finish. This is necessary because sequencing requests would not exercise the
pending limit. It submits 65 reads, expecting 64 admitted and one Busy; this
bounded array/ref collector is not a production request scheduling template.
The sequence also checks:

- Unplaced controller, actual native focus, conditional replacement and stale
  revision; incompatible value/policy and selection validation.
- Directional selection, Preserve endpoint clamping, undo/redo, clear with Reset
  clearing both history stacks, and explicit composition cancellation.
- Recovery-field full-width normalization to `ab12CD34` and mounted-policy limits.
- Disabled focus/history rejection and read-only history rejection, while explicit
  replacement/clear remain available; read-only focus remains permitted.
- Native unmount/remount: new seed `12`, old snapshot/controller rejection, and no
  user Complete event from the programmatic sequence.
- Concurrent close ordering: an already-admitted read can finish before native
  closure; reads issued after local force-close fail Closed. Completion is
  recorded before the runner exits.

No real keyboard, IME candidate panel or VoiceOver action is generated by this
OCaml self-test. The README's Rust `native_otp_input` and macOS
`scripts/test_otp_input.py` describe separate native/foreground coverage.

To add real verification, treat Complete as an intent and launch scoped Eio work
with explicit capabilities. Capture exact value/revision/lifetime and guard any
later reset with `replace_if_unchanged`; keep newer typing on `Stale_revision`
and a replacement editor untouched on `Stale_input`. Handle composition and
policy errors explicitly. Avoid logging snapshots/sexps containing codes even
when the display is masked. Do not perform I/O inside `let%arr`, treat completion
as authentication, or reuse a controller across windows/duplicate placements.
