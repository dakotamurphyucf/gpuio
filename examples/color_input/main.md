# Inline Color Studio: native editing and explicit controller effects

[main.ml](main.ml) mounts one native color editor with hex text, HSLA fields/rails,
palette swatches and programmatic action buttons. It has no separate `.mli`; read
[color_input.mli](../../lib/core/color_input.mli),
[color_value.mli](../../lib/core/color_value.mli) and the
[Eio controller interface](../../lib/eio/color_input.mli) for public contracts.
Start with fixtures and `value_text`, then `component`/`report` and startup, then
the optional `run_self_test`.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/color_input/main.exe
./scripts/gpuio exec _build/default/examples/color_input/main.exe
# Optional bridge/revision/lifetime diagnostic:
./scripts/gpuio exec _build/default/examples/color_input/main.exe --self-test
```

Edit hex, use a channel rail, select a swatch or try Rose glass/Reset/Clear. Normal
launch leaves a 620×600 window open until dismissed. There are no assets or network
prerequisites beyond the [development toolchain](../../docs/development.md).
Only `--self-test` is parsed; there is no background-launch option. Current release
scope is macOS-first; the diagnostic does not replace actual OS keyboard, pointer,
clipboard or VoiceOver validation, and compilation does not qualify Linux GUI.

## Types, initial state and reactive configuration

`color` validates hex via `Color_value.Rgba.of_hex`, wrapping it in `Value.Color`.
`Value.Empty` is a distinct no-color case. Initial is opaque Iris `#7C6FF0`;
`translucent` is Rose glass `#EF6B9580`. `value_text` formats typed values for display;
it does not parse or write native fields. Raising `ok_exn` is reserved here for known
fixtures; external color input should handle constructor errors.

`policy` contains disabled/read_only/hidden/opaque flags, all false. `B.state` creates
this reactive record, shown=true and status text with setter effects. Config's
`let%arr` derives labels, Iris/Rose glass/Mint palette entries, Allow_alpha or
Opaque_only, allow_empty=true and disabled/read-only policy. Native editor value is
not a field in this config: `C.create` separately receives `initial`, applying it
once per mount. Ordinary config changes preserve native state subject to policy.

Bonsai owns policy/visibility and observed text; Rust owns channel values, raw drafts,
composition, selection, undo and preview. `C.snapshot` holds the last accepted native
observation. `on_event` maps every native event snapshot into status, recording event
variants only when self-testing. It does not call `C.set` to echo the observation
back. A second unplaced controller is a diagnostic fixture, never rendered.
`B.map control ~f:C.snapshot` and `B.Edge.on_change` watch typed optional snapshots,
track the current controller and start self-test once mounting has been observed.

`let%arr` combines current reactive inputs to derive the GPUIO view. Buttons receive
`Bonsai.Effect` values, scheduled on activation; they are not executed during view
construction. `C.view` mounts the retained editor once under its controller identity,
with width 380 logical pixels. Outer `Style.Visibility` hides that view without
removing it; shown=false removes it, invalidating its native lease. The hidden flag
is exercised only in self-test; ordinary buttons expose opaque/read-only/disabled
and unmount controls.

## Follow a user edit and an explicit Set

Type a valid color in hex and press Enter. Native editing commits the draft and
queues a Committed snapshot. `on_event` runs `set_status`, Bonsai recomputes the
status view, and GPUIO displays the formatted color. Intermediate drafts/composition
and channel drags remain native. Escape cancels the current interaction; Tab finishes
the old field and moves focus. The application does not reconstruct native text
from the status string.

Click **Rose glass**: its handler is `report (C.set control translucent)`. `report`
uses `Bonsai.Effect.Let_syntax`'s `let%bind` to await a Result, then displays the
snapshot value or typed error. Effect binding sequences async actions; Bonsai
`let%arr` derives values from reactive inputs. Set validates first, replaces fields,
cancels an active edit and clears obsolete history. It emits Observed (and Cancelled
when appropriate), never a user Committed event. **Reset** uses the original mount
seed; **Clear** explicitly sets Empty; **Cancel edit** restores the committed color.
Rejected commands preserve existing edits.

Now enable **Opaque only** after setting Rose glass. The historical alpha value can
remain displayed but `Snapshot.value_allowed` is false; a new Set of that translucent
value fails Invalid_value, and alpha-field focus is blocked. Policy configuration
does not silently rewrite the historical value. Disable/read-only flags restrict user
editing/focus, while explicit Set/Reset/Clear remain available when their values fit
current policy. Hidden and modal-blocked focus requests fail; read-only fields may
receive focus. The [color-input design](../../docs/design/color-inputs.md) explains
editing, revisions and policy behavior.

Unmount/Remount replaces native identity and reapplies the initial Iris seed. A
retained controller may still expose an old snapshot, so `read_snapshot`/commands
must handle Stale_color_input. `set_if_unchanged` fences both native lease and revision,
preventing a delayed effect from targeting a new mount with a coincident revision.
Window closure tears down the controller and resolves further commands as Closed.

## Optional self-test and runtime boundary

`run_self_test` reads an unplaced controller (Not_mounted), checks the initial color,
admits 65 concurrent reads and expects 64 successes plus one Busy, focuses hex,
performs a revision-guarded Set and rejects reuse of its stale snapshot. It exercises
historical alpha/policy, blocked alpha focus, reset, disabled/read-only/hidden states,
cancel, unmount/remount and old-lease guards. It verifies these programmatic commands
produced no Committed events, then reads around window close to assert before/after
ordering. Typed `expect`/`error`/`check` helpers make unexpected responses fail.

`concurrent` uses Expert effect evaluation to issue a batch before replies drain.
`frame`/`settle` request native render callbacks to sequence changes; this is not
physical presentation or real key input. The file has no wall-clock watchdog, so
external test automation must bound a stalled native run. `completed` is asserted
after App.run returns, before Eio prints `GPUIO_COLOR_CONTROLLER_OK` to stdout.
No new diagnostic execution is claimed by this walkthrough; existing acceptance
is recorded in the [OCH-36 evidence](../../docs/evidence/color-inputs-och36.md).

`App.run` owns runtime/window lifetime and asynchronous event delivery. There is
no file/network I/O, producer scope or application polling loop. Diagnostic refs,
Expert callbacks and concurrency/frame helpers are verification infrastructure,
not a replacement for ordinary typed command effects. For a small adaptation,
change palette fixtures through validated RGBA constructors while keeping labels
and alpha policy compatible. For a confirmed application color with draft/cancel
semantics, use the [popup picker](picker.md), rather than copying each inline
preview into a controlled application value.
