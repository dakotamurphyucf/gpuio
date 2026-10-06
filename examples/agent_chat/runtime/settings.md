# Fence native settings drafts with page epochs

[settings.ml](settings.ml) and [settings.mli](settings.mli) compose the window's
Generation, Connection demo, Dates & reviews and Annotation color sheet. Native
editors own drafts; the application stores accepted settings. Connection accepts
any six digits as a local rehearsal: no credential/account/network service is
created. Everything ends when this window closes.

Use the [isolated environment](../../../docs/development.md) from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Settings; commit chunk size with Return, restore a draft with Escape,
preview/commit sliders, apply a score interval, try 123-456 in Connection demo,
then inspect date/color pages. Reset generation preferences opens a second
confirmation. macOS is the v1 target; Linux GUI checks remain
[informational](../../../docs/platform-release-policy.md).

## Accepted model versus native draft identity

Read Page/Modal/Model, `create`, `move`, `toggle`, `current`, `update`,
`number_error`, then `component`. Modal is Closed | Open Page.t | Reset Page.t;
Reset remembers the underlying settings page. Model holds epoch, accepted
Generation_settings, accepted Score_range, optional interval/score previews,
numeric error, step-control layout, canonical OTP value/completed flag and notice.
Initial modal Closed, epoch 0, generation defaults 17 bytes/20 ms, score 0–100,
no previews/errors, side steppers, empty OTP and incomplete connection. Separate
Schedule_settings/Annotation_settings objects retain their accepted window values.

`move` changes modal, increments epoch and clears temporary numeric/slider error/
preview fields. `current t epoch` returns a model only while open/reset and the
epoch matches. `update t epoch f` defers applying `f` to the latest matching
model; old native event/effect closures cannot mutate a new page visit after
closure/navigation. Controller keys settings-<name>-<epoch> deliberately give
native number/slider/OTP controls fresh draft identities on a page epoch change.
Ordinary accepted-model updates do not increment epoch or replace a live draft.

Bonsai expert variables supply reactive model values. `let%arr` derives the sheet
from current observations rather than doing I/O. `match%sub active Schedule` and
Annotation conditionally activate those picker subcomputations; this reactive
branch switching retires their native drafts on page deactivation while their
persistent accepted objects live outside the branch. A closure from `is_current`
adds the outer epoch guard. [Date settings](schedule_settings.md) and
[color settings](annotation_settings.md) explain their own confirmed/draft flow.

## Native controls and application commits

`field` composes validated Form.Field labels/help/errors around native controls;
Presentation.settings_group/banner separate sections. Literal constructors use
Or_error.ok_exn because checked-in configuration respects their domains.

Generation's View.number_input has 4–128 step-one byte domain. `number_error`
classifies Empty/Incomplete/Invalid/Out_of_range/Valid snapshots into help;
Committed Number invokes the pure validated Generation_settings update and
`accept_generation`. Other events change error/notice, not saved chunk size.
Side/stacked/hidden step layouts update config without changing controller key.
The [generation guide](generation_settings.md) traces accepted backend updates.

Interval View.slider uses 10–200 ms/step 10. Drag_started/Preview populate only
interval_preview; Committed saves generation and clears preview; Cancelled drops
preview. Score's two-thumb slider uses 0–100/step 1, storing a committed
[Score_range](score_range.md) separately from drag preview. Apply is disabled
while preview exists and its effect rereads the current guarded model before
calling [Results.filter_scores](results.md). Commit and query Apply are distinct;
a slider frame never starts another dataset transformation by itself.

Connection creates OTP Policy length six, native otp_input with canonical model
code and an epoch-bound controller. Changed snapshots store code and retain
completed only if the code is unchanged; Complete stores code/true and a no-service
notice; Rejected displays a six-digit error; Observed does nothing. Clear empties
code, clears completed and increments epoch, ensuring native draft replacement.
Canonical partial code survives page changes, but native editing session/focus/
undo does not survive destruction. Read the
[OTP contract](../../../lib/core/otp_input.mli) for normalized paste and complete
observations, not an authentication protocol.

## Sheet/confirmation lifetime and interaction trace

View.sheet has stable workspace-settings key, label Workspace settings,
490-pixel extent and dismiss effect guarded by captured epoch. Closed supplies
None, removing native content; Open/Reset supplies scrollable current page.
Page buttons call move with another Open page. A displayed Esc shortcut hint is
not another shortcut registration; native sheet dismissal owns that behavior.

Reset generation preferences changes modal to Reset page without changing epoch,
so the underlying page stays present while a nested alert-dialog blocks normal
interaction. Keep preferences/dismiss restores Open page. Confirm calls
accept_generation G.default, restores Open page, increments epoch and clears
number/slider previews. It preserves score bounds, dates, annotation and OTP;
running conversation streams keep their captured configuration. The
[overlay/view contracts](../../../lib/core/view.mli) provide focus/input semantics,
not application persistence.

For a concrete submit flow: type a numeric draft → native classification event
queues guarded error update → Return produces committed Number → the deferred
validated update invokes on_generation and saves the accepted model → Workspace
updates its backend → a later Send captures that config. Closing/changing pages
before a delayed old commit completes makes its epoch obsolete, preserving the
new page's accepted model. For a score slider: preview is temporary, release
commits the range, Apply adopts a complete Results filter with its own producer
cancellation/generation rules.

No standalone Eio task is owned by this module. Picker native reads/command
replies are asynchronous; result transforms and response streams belong to
separate scoped controllers. Accepted settings/view publication does not prove
physical native editing, IME or painting. Explicit Demo controls can override
the backend independently of saved generation settings; later commit translates
the settings again.

A small adaptation is a new validated setting: add accepted model data/default,
construct the native control with stable epoch identity, distinguish preview/
commit and add guarded updates. Do not mirror per-key native drafts as accepted
configuration or reuse a retired controller key to force a reset. The optional
`python3 scripts/test_agent_chat_settings.py` and dates/colors runner have
[README](../README.md)/[existing evidence](../../../docs/evidence/agent-chat-m5.md)
prerequisites/limits. This source review runs no native settings check.
