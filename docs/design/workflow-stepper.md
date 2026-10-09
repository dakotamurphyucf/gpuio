# Workflow stepper

`Gpuio.Stepper` is application-owned stage navigation, distinct from numeric
increment/decrement. It composes existing native rich buttons and ordinary views;
it adds no Rust workflow owner, bridge message, timer or application scheduler.
The pinned source distinction is recorded in the
[numeric/OTP review](../catalog/numeric-review.md).

## State and requests

A validated model contains at most 64 ordered `Choice` steps, an optional current
`Choice.Id`, and whole-control disable. IDs are stable and labels can change.
Labels must be nonblank and at most 1024 UTF-8 bytes, matching the native rich
button accessible-name limit. Collection duplicate-ID/text limits also apply.

Current must name an existing step, including a disabled one. Explicit `select`
can set a disabled current step; user `apply_request` cannot. `with_steps`
preserves current identity through reorder and clears it when that ID disappears.
Empty models and an absent current are valid.

`Select id`, `Previous` and `Next` requests apply to the latest application model.
Disabled or removed targets are no-ops. Relative requests skip disabled steps,
stop at either end and never wrap. Without current, Next selects the first enabled
step and Previous the last. Multiple queued relative requests therefore advance
from the results of earlier requests, not a stale rendered index.

Status is positional: earlier steps are `Completed`, current is `Current`, later
steps are `Upcoming`. Without current, every step is Upcoming. This reproduces the
upstream stage presentation, not business validation. Revisiting an earlier stage
changes these positional statuses. Applications own validation, permissions,
submission and persistent completion records separately.

```ocaml
let model =
  Stepper.create ~steps ~current:(Some first_step_id) () |> Or_error.ok_exn
in
Stepper.view model ~label:"Publishing workflow" ~on_request:inject ()
```

In Bonsai, `inject` can be a state-machine action producing `unit Effect.t`;
`Stepper.view` then returns `Gpuio_bonsai.View.t`. Apply the request in that
state machine using `Stepper.apply_request` and any application-specific guards.
No extra Bonsai adapter is needed. The gallery contains a working example.

## Presentation and accessibility

`view` supports horizontal/vertical layouts and centered horizontal labels. Each
step has a stable keyed wrapper and a single focusable rich-button trigger.
Indicator/content/connector wrappers have no action handlers. Defaults show step
numbers and labels; pure `indicator` and `content` functions may return passive
ordinary views, including icons. Rich-button validation rejects interactive or
native-owned descendants and enforces its node/depth/style limits. All native
tree/frame budgets still apply to the complete composition.

Appearance controls logical-pixel indicator size, connector thickness and gap,
plus item/indicator/connector and positional-state styles. Defaults use existing
background/foreground/accent/muted theme tokens. Styles merge in documented order;
layout overrides remain caller-owned. A size or style change does not replace
step identity or associated application content.

The root is a labelled navigation region. Native buttons handle Tab/Shift-Tab,
Enter/Space and accessibility activation; disabled steps are skipped. Current-step
metadata is distinct from focus. Localizable descriptions convey one-based
position, total and status, also on platforms whose current-step property is
limited. Custom rich content retains the Choice label as its accessible name;
include meaningful supplementary information in the description formatter.
Arrow-key roving is not substituted for the ordinary navigation-button contract.

Selecting a stage does not mount, unmount or reset its page. The application
chooses its content lifetime. The gallery intentionally keeps a native notes
field outside the stepper, preserving its draft/history across all stage and
presentation changes. Whole-control and per-step disable use one reducer with
selection, so queued requests see current policy.

## Native renderer repair

The public composition exposed a default-debug-stack overflow during the first
TestPlatform draw. Nonrecursive event/focus/probe decoration previously lived in
the large recursive retained-tree builder. It now lives in `node_actions.rs`,
preserving behavior while reducing temporary stack use at each recursive level.
The same full fixture passes without increasing stack limits. This is evidence
for that regression, not qualification of every permitted tree depth, physical
desktop behavior, or a diagnosis of the separate black-startup-window issue.

See [local evidence](../evidence/workflow-stepper-och41.md). Physical macOS visual,
keyboard and VoiceOver acceptance and release/resource validation remain open.
