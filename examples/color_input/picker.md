# Popup color picker: a native draft and an application-owned accent

[picker.ml](picker.ml) uses `Gpuio_eio.Color_picker` to separate confirmed application
color from a native editing draft. It has no separate `.mli`; public contracts are
[Eio color_picker.mli](../../lib/eio/color_picker.mli),
[pure picker policy](../../lib/core/color_picker.mli),
[color_input.mli](../../lib/core/color_input.mli) and
[color_value.mli](../../lib/core/color_value.mli). Read fixtures, `format`/`error_message`,
`component` and the keyed controller graph before diagnostic `exercise`.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/color_input/picker.exe
./scripts/gpuio exec _build/default/examples/color_input/picker.exe
# Optional command/session/lifetime diagnostic:
./scripts/gpuio exec _build/default/examples/color_input/picker.exe --self-test
```

Open the color trigger, edit hex/channels and Apply, or use Cancel/Escape/outside
dismissal. Try Reset color, opaque/read-only/disabled policies, dialog and right-edge
placement. No external files or services are used. Toolchain prerequisites are in
[development](../../docs/development.md). Only `--self-test` is parsed, with no
background-launch flag. The [README](README.md) links separate native scripts and
[existing OCH-36 evidence](../../docs/evidence/color-inputs-och36.md); this guide
adds no OS input, focus, pixels, Linux desktop or VoiceOver acceptance.

## Confirmed value, reactive policy and native draft

Initial confirmed color is translucent Iris `#7C6FF080`; replacement and draft-color
fixtures are Rose `#EF6B95` and Mint `#54C6A2`. `color` constructs validated
`Color_value.Value.Color`; Empty is a separate typed no-color value. `format` emits
hex or “Choose color” for the trigger and confirmed readout, without writing editor
text. Fixture `ok_exn` wrappers should become explicit validation for external input.

`B.state` creates value, flags, placed/active=true and dialog/right-edge=false with
setter effects. `flags` starts disabled/read_only/restricted=false. Config's
`let%arr` derives Accent color labels, Iris/Rose/Mint palette, allow_empty=true and
alpha policy. Restricted means Opaque_only and is exposed by the Opaque only button.
`on_change` records a successful confirmation and runs `set_value`; it is the only
picker callback that changes confirmed color. Native field commits and previews
change the popup draft, not the confirmed application value.

`B.map active` supplies Int map entry 0 or an empty map. `B.assoc` constructs a
keyed `P.create` subgraph only for that entry. Removing it truly deactivates the
picker graph; placed=false merely removes its rendered view while the computation
remains. `B.map2` tracks open/draft/error and application value; `B.Edge.on_change`
stores latest controller/value in refs for verification. `B.Edge.lifecycle` starts
the test once on activation, and the outer `let%arr` derives views from current
inputs. Reactive derivation and scheduled effects are distinct: a setter or
`P.confirm` effect runs in response to activation, not while building the view.

## Trace confirmation, cancellation and policy changes

Open the formatted Iris trigger. `P.view` requests a new opening and mounts a native
color editor seeded from the confirmed value. `P.draft` remains None until a queued
native observation arrives; popup opening or a frame callback alone is not readiness.
Edit the draft to Rose: native text/HSLA state changes, while “Confirmed” stays Iris.
**Apply** asynchronously reads native state and validates the current opening,
application value, alpha/empty policy and read-only state. On success it calls
`on_change Rose`, which updates Bonsai value and closes the popup; `let%arr` derives
the new trigger/readout. A valid noncomposing text preview may apply without a
separate Enter, but composing/invalid text or an unfinished drag cannot confirm.
`can_confirm` is a visual hint, not a substitute for this fresh validation.

Cancel/Escape/outside dismissal discards the draft without calling `on_change`.
In a popup, Escape dismisses the whole opening; the [inline example](main.md) instead
uses Escape to cancel its current native editing interaction. A captured old cancel
cannot close a later opening. Reset color directly changes the confirmed reactive
value, invalidating a draft based on obsolete state. Read-only may retain a draft
but rejects Apply; disabling closes it and suppresses reopening while disabled.

If Opaque only makes the historical translucent confirmed color disallowed, opening
starts Empty because this config allows it. This fallback does not mutate the
confirmed Iris; Cancel leaves that historical value intact. An alternative config
that disallows Empty would seed original RGB with opaque alpha, or opaque black
for a disallowed Empty. Current policy is rechecked on every Apply. The typed
`error_message` cases describe composing, invalid draft, drag, stale session/draft
and native errors for the notice.

The popup config is width 380 with outside-pointer dismissal. `P.view` appears only
once: in a left/right-aligned row, or inside `View.dialog` with key `picker-dialog`
and 440-pixel width. Moving placement/dialog first runs `P.cancel` with the state
effect via `E.Many`. Dialog dismissal/Done cancel the picker and clear dialog state.
Native adapters calculate bounded placement, focus restoration and modal dismissal;
this file supplies labels/configuration rather than screen-position math.

## Diagnostic sessions, native mounts and cleanup

`exercise` reads the latest controller ref, opens and settles frames, then `ready`
waits for open+Some draft with at most 120 frame requests. A captured controller
value does not acquire later reactive observations. The diagnostic `replace` helper
uses `P.command (Set { value; if_revision=None })` to change only the draft; it never
writes the confirmed model directly. `Bonsai.Effect.Let_syntax`'s `let%bind` sequences
these async requests and typed assertions, unlike graph `let%arr`.

The diagnostic checks closed confirmation, Apply/Cancel, stale-opening commands,
view remount restoring the confirmed seed, old native lease rejection, cancellation
during an in-flight confirmation and keyed deactivation/reactivation. It then checks
read-only, external reset, restricted historical Empty fallback, disabling, Empty
Apply and close ordering. `cancel_during_confirmation` uses Expert evaluation to
admit confirmation and cancellation before its native read reply can commit. Old
replies must not change confirmed color or close a later opening. After window close,
confirmation on the captured final controller returns Native Closed. Successful
completion asserts `completed` and prints `GPUIO_COLOR_PICKER_PUBLIC_OK`.

These are bridge/session diagnostics, not real pointer or IME input. `frame`/`settle`
observe native render callbacks, not physical display timestamps; readiness is
checked separately. There is no in-file wall-clock watchdog, so an external runner
must bound a stalled window. The [existing evidence ledger](../../docs/evidence/color-inputs-och36.md)
separates diagnostics from physical checks; no new run occurred for this guide.

`App.run` opens a 760×600 window and owns runtime/controller cleanup; deactivation
and window close invalidate draft lifetimes and fence late results. There is no
application file/network I/O, Eio producer or polling timer. `Eio_main.run` after
App.run only writes the success marker. For a small adaptation, change the initial
accent or add validated palette entries, retaining value-policy compatibility.
Keep `on_change` as the explicit confirmation boundary and one view per controller;
never make native previews write the confirmed model or reuse a captured opening
for a new draft.
