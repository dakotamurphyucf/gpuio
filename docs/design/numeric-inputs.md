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
