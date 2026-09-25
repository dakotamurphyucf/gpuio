# Numeric, range, stepper and OTP inputs (OCH-34)

Status: implementation started. No OCH-34 capability is advertised. Draft the
public contracts before native integration; acceptance remains the full ticket.

## Pinned implementation review

Inspected the vendored gpui-base sources from GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, already adapted to our pinned GPUI.
No dependency change is needed to evaluate these sources.

- `vendor/gpui-base/src/slider.rs` supports single and two-thumb ranges, horizontal/
  vertical axes and linear/logarithmic mapping. Its value model uses f32. Pointer
  rounding is anchored at zero rather than min; release has no cancel event.
  Root AX increment/decrement calls `set_value(f32)`, which changes a range into
  a single value, and does not emit the user Change/Release events. The root
  exposes only the end value; independent keyboard/AX range thumbs are missing.
- `vendor/gpui-base/src/number_input.rs` composes the existing InputState with
  non-focus-stealing step buttons and a SpinButton root. Default behavior installs
  a number mask and parses f64. Invalid/empty text steps from zero. Its decimal
  formatting derives precision from strings, which is not a sufficient contract
  for exponents, finite overflow and intentionally incomplete drafts.
- `vendor/gpui-base/src/otp_input.rs` handles numeric key append/backspace with
  Change/Complete events and optional masking. It normalizes full-width digits,
  but lacks selection, paste distribution and a native IME text-input handler.
  Programmatic strings are deliberately unfiltered, even beyond configured length.

These are useful implementation references, not complete bridge adapters. Reuse
our established native editor/ownership infrastructure for text. Implement the
slider's bounded numeric/interaction model explicitly in GPUIO so single/range
modes keep their identity and all input routes share one mutation path. Do not
inherit the candidate's accidental behavior as a public promise.

## Shared numeric domain

`Numeric.Domain` is a validated closed interval with finite f64 min/max and a
positive finite step. Equal bounds are legal and yield one fixed value. For a
nondegenerate interval, require a finite span, at most 2^40 step intervals, and
step >= eight machine epsilons times the largest bound magnitude. This rejects
configurations whose grid cannot be resolved reliably; it is not exact decimal
arithmetic. Step larger than the span remains useful and selects the endpoints.

The grid is anchored at minimum, and maximum is always selectable even if it is
not a regular grid point. Normalization clamps and chooses the nearest adjacent
grid/end point, preferring the greater value on an exact binary-float tie. A step
first normalizes, then selects the adjacent point in its direction; endpoints
saturate. Reject nonfinite input. Canonicalize numeric zero without rewriting
an editor draft. Calculations are bounded, with no allocation proportional to
the number of steps. OCaml and Rust implement the same rules and independent
fixtures/behavior tests check agreement.

`Numeric.Draft` classifies the original text as empty, incomplete, invalid,
finite in-range, or finite out-of-range. The grammar is ASCII decimal with an
optional exponent, sign and surrounding ASCII whitespace; a dot is the decimal
separator. Prefixes such as `-`, `.`, `1e` and `1e-` stay incomplete. `1.` is a
valid number whose draft remains unchanged. Reject hex, separators/underscores,
NaN/infinity, malformed suffixes, overflow and more than 4096 bytes. The draft
classification never issues an editor replacement or moves the caret.

## Native contracts to implement

Slider: one or two native-owned values; snapshots are observations, never writes.
Separate initial values from revision-checked replace/reset commands. Both thumbs
remain ordered and clamp at each other rather than swapping identity. One Tab stop
per thumb with distinct accessible names/ranges; keyboard, pointer and AX actions
use the same normalization. Support both orientations and linear/log mapping
(positive minimum for log). Native drag previews are coalesced; begin/final commit/
cancel are ordered and never coalesced away. Escape, policy/bounds changes,
visibility loss, modal exclusion and unmount cancel an active drag. Bounds updates
cancel first, then normalize the retained value under the new domain.

Numeric input: keep native draft/selection/composition and a last committed value
separate. Native editing may be incomplete or invalid. Enter/explicit commit
validates then normalizes; invalid/incomplete input stays visible and reports a
rejected commit. Empty commits are explicitly optional. Escape restores the last
committed value. Config/observation updates do not replace draft or caret. Step
buttons, keyboard repeat and AX stepping apply natively, preserve editor focus,
and use the shared domain. Do not commit through active IME composition. Specify
bound changes and undo behavior with the editor integration before freezing API.

OTP: retain one native text-editing session with a segmented visual presentation,
selection/navigation and IME support; do not create one editor per cell. Bounded
length (1..32) and explicit alphabet policy. Decimal digits default; optional
ASCII alphanumeric. Normalize full-width allowed characters. Paste can remove
ASCII whitespace/hyphen separators, but unexpected characters or overlength
insertions reject atomically. Change and Complete are distinct ordered events;
completion says only that input is full, not that authentication succeeded.
Explicit revision-checked replacement/clear commands; observing or changing
mask/disabled state never resets the value. Final API and native tests must cover
programmatic validation, selection/paste, read-only, hidden idle and disposal.

## Acceptance sequence

1. Shared numeric domain/draft rules and paired codec validation.
2. Native single/range slider ownership, input, accessibility and coalescing.
3. Native numeric editor/stepper integration and transient draft/IME behavior.
4. Segmented OTP editing, commands and event ordering.
5. Public Core/Bonsai/Eio examples, actual local macOS tests and lifecycle/budget
   evidence; full local checks. Advertise capability only after this acceptance.
6. Consolidated required macOS/Linux CI and merge. The polished chat integration
   remains OCH-46; Linux desktop GUI acceptance remains OCH-17.

## Slider ownership and event contract

The Core `Slider` contract distinguishes configuration, initial value, immutable
snapshots and explicit commands. A native owner mounts in single or range mode;
ordinary configuration or initial-value rerenders cannot change that mode or
reset its values. Remount to change mode. Initial/replacement values normalize
under the configured numeric domain. Range thumbs retain lower/upper identity
and stop at each other; they never cross or swap.

Snapshots carry a monotonically increasing native revision, current preview,
last committed value and optional dragging thumb. Outside a drag, current and
committed values match. Start/Preview/Committed/Cancelled distinguish a gesture
from ordinary Observed state. Labels can change during a drag; axis/scale/domain
or disabling/read-only transitions cancel first. Cancellation restores the prior
committed value. A domain change then normalizes that committed value and emits
Observed under the new domain. The cancellation snapshot therefore describes the
old domain, followed immediately by the new-domain observation.

Keyboard/AX mutation interrupts a pointer gesture with Cancelled before applying
the discrete mutation. Rejected/nonfinite/wrong-thumb input does not interrupt it.
Replace optionally checks the observed revision, preserves mode, cancels an active
drag and emits Observed; it is allowed programmatically under disabled/read-only.
Focus remains subject to native focus/visibility/modal gates. Read_snapshot is
non-mutating. Revisions fence native state changes, including configuration and
drag lifecycle; stale guarded commands fail atomically. Overflow also fails
before mutation. Intermediate previews may coalesce only inside their gesture;
start/final/cancel boundaries and command responses remain ordered. The mounted
bridge must apply node-generation and current-handler fences in addition to the
model's revision checks.

The retained bridge appends Kind 37, Set_slider operation 43 and Slider_event
event 44 without changing older tags. `View.slider` uses its controller key as
the node key and rejects duplicate controller ownership anywhere in one window.
Core snapshots retain the originating window and node generations behind their
abstract interface. Reconciliation preserves the native-revision fence through
callback refreshes and pending tree updates, admits only the current owner and
handler, and rejects repeated or backwards observations. Invalid observations
cannot advance that fence. A remounted owner starts a new fence.

Observation handlers remain bound when disabled/read-only. Policy and domain
changes do not rotate the handler or filter historical cancellation against the
new domain: that would lose the old-domain cancellation described above. Native
input must enforce current policy before creating a user mutation. Transport
may replace only adjacent previews for the same window/node/handler/tree revision,
dragging thumb and committed value, with increasing native revisions. Every
discrete lifecycle event, response and unrelated event remains a barrier. Full
queues explicitly reject additional discrete input through the existing overload
path; coalescing does not silently discard commits.

Two range thumbs expose separate native accessible values/bounds/actions and Tab
stops. Arrows step, Page Up/Down step ten times, Home/End go to the available bound,
and Escape cancels an active drag. Vertical values increase upward. Mapping is
computed in f64 and converted to bounded fractions before native pixel geometry;
logarithmic mapping must avoid ratio overflow and loss of narrow positive spans.

## Public slider controller and correlated commands

`Gpuio_eio.Slider.create window ~config ~initial graph` creates one Bonsai-owned
controller key; `Slider.view` places it once. Rust owns the changing values and
active gesture. `snapshot` is the last observation, not a controlled value prop.
Optional `on_event` receives ordered native lifecycle events. Configuration can
vary reactively; `initial` only seeds a new native placement.

`read_snapshot`, `replace`, `replace_if_unchanged`, `cancel_drag` and `focus thumb`
return Bonsai effects with a typed result and exact native snapshot. The transport
appends Message Slider_command tag 14 and Event Slider_result tag 45. Each request
carries a positive correlation plus exact window/node generations; each response
is Applied snapshot or Failed error. Native_failure is appended to the error
variants. No callback or borrowed Rust object crosses the bridge.

The application bounds pending slider requests to 64. Responses use reserved
mailbox capacity independently of input pressure and keep a window slot occupied
until drained. Closing/shutdown completes pending requests with Closed and late
replies cannot complete them again. The controller applies a reply observation
only when its expected lease is still current; older revisions cannot roll state
back. A saved controller never silently retargets a remounted owner.

Read_snapshot is non-mutating and works for hidden, read-only or disabled owners.
Replace normalizes and preserves mode, optionally checks the native revision,
and cancels an active drag before its Observed event. It also works while hidden,
read-only or disabled. Rejected commands leave a drag untouched. Cancel_drag
restores committed values, releases capture and reports Programmatic cancellation;
when idle it succeeds without incrementing the revision. Focus validates the
requested thumb and current disabled/visibility/modal policy. Replies update the
controller's snapshot but do not duplicate the user lifecycle callback.

The runnable `examples/numeric` application uses only public Core/Bonsai/Eio APIs.
Its self-test deliberately keeps an old controller through unmount/remount and
checks both stale native revisions and stale placement identity.

### Slider appearance

`Style.Foreground` is the slider accent: selected rail, thumb fill and native
focus ring all use the resolved foreground at paint time. The unselected rail
uses that same color at 25% opacity. The default accent is `#6688ff`; applications
can override it with their theme tokens and ordinary hover/focus/disabled style
rules. Focus styles apply when either thumb is focused; a separate outer ring
identifies the particular focused thumb without relying only on a color change.
The host's standard disabled opacity applies once to the whole control. Read-only
sliders retain their appearance and focusability while rejecting user mutation.

Background, border and layout styles decorate the outer control. Geometry is in
logical pixels: 4px rail, 12px solid thumb, 20px thumb target/focus ring, 10px endpoint
inset, and a default minimum of 24px on each axis. Width/height change the available
travel, not the domain. The native renderer scales these with the display density.
Painting uses a constant number of primitives per slider and has no idle timer.

Pointer mapping uses the actual rail bounds produced by native layout, including
asymmetric border/padding refinements. If those bounds change during capture,
the drag cancels with `Interrupted` and restores the committed value, even when
the outer control's size has not changed. Late release cannot commit the old drag.

## Numeric editor and stepper implementation target

The following refines the accepted draft/value contract for the next OCH-34
implementation. It is an API target, not a claim that `Number_input` exists yet.

### Ownership and public shape

Use one native single-line `InputState` per `View.number_input` placement. The
editor owns draft text, UTF-8 selection, composition and bounded undo history;
a native numeric owner holds its validated configuration, committed value and
observation revision. Step buttons share that owner and preserve editor focus.
Do not use the candidate NumberInput's mask/parse-to-zero path.

The Core module will be `Number_input`, with `Value`, `Config`, `Revision`,
`Snapshot`, `Event`, `Command` and `Command_error` submodules. `Value` distinguishes
`Empty` from a validated finite `Number`; constructors reject NaN/infinity.
`Config` owns `Numeric.Domain`, labels, placeholder, empty-commit policy,
`Hidden | Sides | Stacked` step controls, disabled/read-only and initial autofocus.
A public `Gpuio_eio.Number_input` Bonsai controller supplies one stable placement,
observations and correlated commands, following the slider/editor lease rules.
Snapshots are owner-bound and opaque, exposing the draft, its current
`Numeric.Draft` classification, last committed value, selection/composition,
focus and a numeric observation revision. They are never fed back as replacement
properties. `initial` seeds the native owner only once.

An empty initial value means no committed number yet, including required fields.
`allow_empty` controls whether a user/explicit Commit may accept Empty. A config
change that makes an existing empty value required does not invent a number or
rewrite the draft; the next empty Commit is rejected. Explicit `Replace_value`
may reset to Empty, including a required field, just as mounting an empty field
can. It emits an observation, not a successful user-commit event.

### Editing, commit and stepping

- Ordinary text edits, paste, selection and marked text stay native. Enforce a
  4096-byte draft bound using the existing editor's edit/IME/history limits.
  Unicode drafts may be displayed even though the numeric grammar is ASCII.
  Classification is not an edit filter: `-`, `1e-` and invalid drafts remain
  visible. Numeric parsing never silently substitutes zero for invalid input.
- Enter outside composition and `Commit` validate, normalize to the current
  domain, and format the resulting finite value as a round-tripping decimal.
  Valid out-of-range numbers clamp through `Numeric.Domain.normalize`.
  Empty commits succeed only when allowed. Incomplete/invalid or disallowed
  empty commits report a typed rejection and preserve draft/selection.
- Focus loss alone does not commit or rewrite text. Applications that want a
  form-level submit use the explicit Commit operation and its typed result.
- Escape first follows the existing editor/IME marked-text handling without
  restoring the committed value in that same keystroke; outside composition it restores the committed value's text. Explicit Cancel
  during composition rejects with `Composing` rather than discarding marked text.
- Up/Down, step buttons and AX increment/decrement normalize a valid numeric
  draft then advance one shared-domain point, saturating at bounds. An empty
  draft starts at normalized zero without an extra step. Incomplete/invalid
  drafts reject stepping and remain untouched. A successful step commits its
  value and places the caret at the end. No stepping runs through active IME.
- Successful normalization/stepping/cancel replacements form individual native
  undo transactions. Undo/redo restore draft and selection, not the committed
  application value; restored drafts are classified again and require a new
  commit. Explicit replacement can choose Record or Reset history, as text input
  already permits. This distinction must be demonstrated in the public example.
- Domain changes normalize the stored committed number and reclassify the draft,
  preserving text, caret, composition and history. They produce an observation,
  not a user commit. Styling, labels and routine observations never reset text.
- Disabled/read-only suppress user mutations and stop pointer repeat. Explicit
  value/draft replacements remain possible, as in existing editor/slider APIs.
  Programmatic Commit/Cancel are explicit edits; programmatic Step follows the
  same disabled/read-only policy as user stepping. Read/focus retain normal
  native gates. All mutating commands other than focus/read reject during composition.

The numeric observation revision advances for every exposed state change and
commit/rejection/cancel event, including semantic changes without an editor text
edit. This gives repeated commit attempts distinct ordered identities. The inner
editor's text revision remains an implementation detail. Command/step/commit
paths must reserve revision capacity before mutating either numeric state or
editor text. If an ordinary editor observation exhausts the numeric sequence,
fault window input rather than publishing inconsistent state or rewinding user
edits. Stale/foreign guards and invalid commands fail without touching either state.

### Commands, native integration and validation

Provide Read_snapshot, Focus, Select, Undo, Redo, Commit, Cancel, Step,
Replace_draft and Replace_value with explicit selection/undo policies and
optional numeric revision guards where replacement can race edits. The Eio
`replace_if_unchanged` helpers additionally check the window/node lease. Separate
observations from committed/rejected/cancelled events; coalesce only adjacent
ordinary change observations, preserving semantic boundaries and command replies.

Native subscription handlers must reconcile the number owner with the live editor
before publishing or serving a command. Commit/step/replace capture their final
editor snapshot synchronously and suppress a later duplicate ordinary-change
notification. No synchronous OCaml call occurs from an editor action, IME delegate,
layout or paint. Reuse narrow native editor helpers; do not expose an unrestricted
second text-input controller for the same numeric placement.

Use semantic numeric value/min/max/step and increment/decrement actions alongside
the editable text field's native text selection/IME interface. Invalid drafts need
accessible validation feedback without claiming a numeric value parsed from
invalid text. Test the actual macOS role/value/action mapping; source declarations
alone are insufficient. Pointer-repeat tasks must be weak-owner scoped, stop on
release/cancel/policy/visibility/window loss, and never wake an idle field.

Implement and validate in this order: Core/Rust contracts and independent codecs;
reconciliation and a native numeric model; native editor/stepper integration;
Bonsai/Eio controller; public examples and actual paste/IME/selection/undo,
keyboard/pointer repeat, AX, stale-guard and lifetime acceptance. Reuse the
existing shared domain/draft tests and include transient `-`, exponent prefixes,
rounding boundaries, overflow and domain updates during marked text.
