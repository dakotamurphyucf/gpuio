# A numeric draft is separate from its committed value

[number.ml](number.ml) builds three presentations of the same native numeric
editor: side steppers, stacked steppers and keyboard-only hidden steppers. The
[README](README.md) contains exact `number.exe` build/run commands and separate
macOS diagnostics. [dune](dune) isolates module `number` for this executable and
links Core/GPUIO/Bonsai/Eio with Jane Street/Bonsai PPX. No assets, credentials or
external work are involved; setup uses the
[development toolchain](../../docs/development.md) and current
[platform policy](../../docs/platform-release-policy.md).

Read the aliases/helpers, initial part of `component`, final view, and launcher;
then revisit the self-test callback. `value` validates finite `N.Value` numbers;
`domain` validates bounds −2..8 and native step 0.5. `N.Value.Empty` can represent
an unfilled required field; domain normalization occurs on mount/replacement or
commit. `draft` explicitly replaces text with undo `Record`; `replace` sets a
committed numeric value and formatted draft with undo `Reset`. The distinction
is central: typing `-` or `1e-` is a legitimate unfinished draft, not a committed
numeric value. See [Number_input](../../lib/core/number_input.mli) and
[Numeric](../../lib/core/numeric.mli) for grammar and binary-float normalization.

`component` constructs a persistent Bonsai graph. `B.state` owns shown true,
mount value 1.5, disabled/read-only false and instruction status; `B.state_opt`
starts mount draft absent. Typed `N.Value.equal`/`N.Draft.equal` compare seeds.
These are application policy and next-mount seeds. Rust owns live text,
selection, IME, undo and committed value. Reactive values change over the graph
lifetime; `let%arr` reads current ordinary values to derive config/view.
Its `and` bindings are dependencies, not threads. State stays allocated outside
that repeated derivation; setter effects execute only on interaction.

`Controller.create` builds the primary editor with seed 1.5 and a reactive config.
The early `mode label step_controls` helper creates the stacked and hidden fields
with seed 2 and constant `B.return` config. Default controls are Sides and default
step mode is Native: user stepping applies the configured domain step in Rust.
This example does not use application `Step_requested` resolution or `Step_task`.
An additional controller is deliberately never placed, solely to test
`Not_mounted`. All actual editors are placed once. Changing disabled/read-only
policy preserves draft; a disabled field cannot focus and read-only can focus.

The later local `mode ?initial ?initial_draft title controller` is a **view**
helper, distinct from the earlier controller-construction helper with the same
name. It displays `Snapshot.draft` and `Snapshot.committed`, and places a 360×44
logical-pixel `Controller.view`. Its optional seeds override the next mount
only. `mount_value` and `mount_draft` changes cannot overwrite an active draft.
`row`/`View.column` provide validated layout spacing. `report` uses
`E.Let_syntax`/`let%bind` to await commands then update status to a revision or
typed error. Commit, Restore, Reset value and Read state call explicit controller
operations; observations never cause automatic replacements.

For a concrete trace, select the side-stepper text, type `99`, then press Enter.
Native editing first produces draft `99` while committed value stays unchanged.
A native commit parses and clamps it to 8, formats draft `8`, updates committed
value and emits an ordered semantic commit event. The controller receives native
observations asynchronously; Bonsai derives “Draft: 8 · Committed: …”, and GPUIO
submits the updated view. Native input/selection/composition stays in Rust.
Pressing Enter with `-` rejects `Incomplete` and retains text/selection; Escape
restores committed formatting. Up/Down and native step buttons step and commit;
losing focus does not implicitly commit. Undo/redo restore drafts/selection,
not the committed value. Replies/transactions do not prove physical display.

`App.run` owns GPUI on the OS main thread and one OCaml Eio UI domain for startup,
graphs and effects. It mounts a 720×680 window. No application file/network
producer or streaming cancellation exists here. Close uses force-close
`App.Window.close` and cancels window ownership; use `request_close` for real
unsaved-draft decisions. See [App](../../lib/eio/app.mli) and the
[controller contract](../../lib/eio/number_input.mli).

## Optional command/lifetime diagnostic

`_build/default/examples/numeric/number.exe --self-test` opens a window and prints
`GPUIO_NUMBER_PUBLIC_OK` on success. `observed` derives a tuple of all three
optional snapshots; `[%equal: ...]` supplies typed equality to `B.Edge.on_change`.
The test waits until all are present before capturing controllers, guarded by
`started`; `latest` tracks a new primary lease. `on_event` records native events
only in self-test mode. `expect`, `error` and `check` are assertion helpers, not
production error recovery. `frame` uses `E.Expert.of_fun` to await
`App.Window.request_frame`; `settle` sequences two callbacks after policy/mount
changes. These are native render observations, not physical presentation.

The sequence tests unplaced `Not_mounted`, guarded replacement and stale revision;
incomplete commit/recovery, `99` clamping and decrease to 7.5; selection/history;
UTF-8 byte-boundary rejection in `é`; newline and draft-size limits; syntax and
required-empty rejection; empty stepping to normalized zero. It checks disabled
focus/step rejection while explicit value replacement succeeds, read-only step
rejection with focus allowed, and identical command routes for stacked/hidden.
The mount-seed test changes value to 4.5 and a different draft while live `1e-`
stays untouched, then remounts with committed 4.5 plus draft `1e-` atomically.
Cancel restores `4.5`; old snapshots/controllers fail `Stale_input`. Recorded
Rejected/Cancelled/Committed events are checked, then close rejects later reads.
The diagnostic uses programmatic command events, not real OS keyboard or AX
acceptance. Separate native/foreground tests are linked in the README.

To accept empty fields, change config with `~allow_empty:true` and distinguish
`N.Value.empty` from zero in application logic. To persist unfinished input,
store validated `N.Draft.t` separately from committed `N.Value.t` and use both as
mount seeds; never feed every observation into replacement. For async resets,
`replace_value_if_unchanged` fences native lease plus numeric revision, preserving
newer edits/remounts. Selection offsets are UTF-8 bytes. Run I/O with explicit
Eio capabilities outside `let%arr`, and handle typed rejection/policy/lifetime
errors without treating every draft as immediately valid committed data.
