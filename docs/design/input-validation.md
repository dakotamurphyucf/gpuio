# Ordinary-input validation

OCH-41, 2026-10-01. **Regex preparation and native single-line edit filtering are
implemented locally, with an authored gallery example.** Physical desktop,
performance/resource and final catalog acceptance remain open.

## Preparation boundary

`Input_validation.Regex.Source.create` constructs immutable source data. Its
optional `matching` defaults to `Whole_value`; `case_sensitive` defaults to true.
The constructor checks at most 2,048 UTF-8 bytes and rejects NUL. Empty expressions
are allowed. It deliberately does not claim that Rust will accept the syntax.

`Gpuio_eio.Input_validation.prepare_regex source` returns
`(Input_validation.Regex.t, Input_validation.Error.t) Result.t`. It needs an Eio
context but no GPUI application, transport or window. Expected errors distinguish
invalid source, invalid syntax, excessive compilation size and malformed native
responses. Diagnostics contain at most 1,024 UTF-8 bytes. Cancellation propagates
as cancellation rather than becoming a validation failure.

The checked value contains only source data. The temporary Rust regex is dropped
before preparation returns; there is no cross-runtime object handle or callback.
`Expert.checked_source` is an internal adapter escape hatch, not a trust boundary:
native policy ingress validates and compiles the source independently.
Preparation must not become a synchronous operation during Bonsai evaluation or
GPUI paint/layout/input callbacks.

Preparation uses an Eio system thread. The native export bounds and copies the
request before releasing the OCaml runtime for parsing/compilation. A shared Eio
mutex admits one preparation at a time through this public adapter. Waiting
callers are cancellable; admitted work is cancellation-protected until its worker
returns. The slot is then released before cancellation is checked again, so a
cancelled caller receives no result and cannot release a slot while its job still
runs. This bounds active jobs, not the number of application fibers waiting to
prepare. Applications should prepare configuration once and reuse the checked
source; do not compile for every keystroke or render.

## Regex semantics and limits

The pinned Rust dependencies are `regex-syntax` 0.8.11 and `regex-automata` 0.4.18,
already present transitively in the repository lockfiles. Rust syntax supports
Unicode classes and inline flags; lookaround and backreferences are unsupported.
The native compiler limits syntax nesting to 32, each configured NFA/full-DFA/
one-pass-DFA size to 256 KiB, and each configured hybrid cache to 256 KiB. These
are engine limits, **not a 256 KiB bound on total process allocation**. Parser
temporary storage, multiple automata and allocator overhead are separate. Syntax
and size checks are not a measured input-latency or resident-memory guarantee.

User capture groups are parsed normally but their result slots are omitted from
compiled automata (`WhichCaptures::Implicit` keeps only the whole-match slots).
Validation exposes no captures. This avoids multiplying fallback-engine state
tables by the number of user groups. A regression with 200 optional groups and a
long Unicode input reproduced a 5,250,111-byte cache before that change; the
repaired populated cache stays below the fixture's 1 MiB limit. That fixture is
not a universal cache bound or a substitute for per-editor resource accounting.

Whole-value matching adds absolute start/end assertions to the parsed expression
tree. This preserves alternatives such as `a|ab` matching `ab` and cannot be
weakened by a multiline flag. Wrapping the source text itself would mishandle a
trailing free-spacing comment; checking only the first match's span would reject
valid longer alternatives. Substring mode uses the expression without those
additional anchors. Case-insensitive defaults can be overridden by explicit
inline flags, following the pinned Rust dialect.

The paired preparation protocol is separate from view operations. Native decoding
bounds source allocation and rejects invalid tags, booleans, UTF-8 and trailing
bytes. OCaml decoding checks the diagnostic's declared length before allocating
it, including truncated/oversized length prefixes. Both sides have independent
byte fixtures. No bridge capability or editor-policy operation is advertised by
this preparation work alone.

## Native edit-filter contract

`Input_validation.regex ?allow_empty checked_regex` creates an immutable filter.
Pass it to `Text_input.Config.create ~edit_filter ...` for an ordinary single-line
input. Multiline configuration is rejected. The name is deliberate: this is an
edit filter, not a certificate of complete form or submission validity.

- `allow_empty` defaults to true, accepting empty text independently of the
  expression. With false, the expression decides whether empty text matches.
  Choose a prefix-compatible expression for incremental typing; a complete email
  address regex is usually inappropriate as an immediate edit filter.
- Native formatting runs first, then the filter checks the resulting formatted
  text. The existing UTF-8 offsets and caret mapping refer to displayed text.
  Explicit replacement/AX SetValue must already contain formatted text and must
  satisfy the current filter before text, selection or history is mutated.
- Mounting a seed or changing a filter preserves the draft, selection, focus,
  revision and history. An incompatible draft remains editable until repaired;
  the recovery path does not silently discard or normalize rejected text. Once
  valid, an interactive edit that would make it invalid is rejected.
- Undo/Redo restore exact history even when it predates the current filter.
  Provisional IME text may differ from the rule. Commit uses the current policy;
  rejection from a valid precomposition value restores that value and directed
  selection, without retaining provisional undo entries. Cancellation follows
  the existing native composition transaction.
- Submissions still return the exact native draft outside composition. They do
  not imply that the draft matches a changed filter or satisfies business rules.
  Applications own complete-value/server validation and stale-result handling.
  There is no synchronous OCaml predicate from editing/layout/paint.

The unpublished paired epoch-3 protocol appends Op77 `Set_editor_validation` /
`SetEditorValidation`. Its optional rule contains bounded regex source, matching
mode, case flag and `allow_empty`. Legacy EditorConfig bytes remain unchanged.
Both language runtimes must be rebuilt together. Native ingress recompiles source
independently of preparation, rejecting syntax/resource failures atomically. Only
ordinary Input nodes accept the operation; picker query descendants are excluded,
including descendant-only transactions. Numeric and OTP policies are unchanged.

## Retained ownership and accounting

Each accepted policy owns its immutable rule, one compiled regex and one explicit
matching cache. Tree and editor retain the same Arc. Reapplying an equal rule
reuses that object; rendering/style changes do not compile another expression.
Matching uses the explicit cache, avoiding the regex object's implicit per-thread
cache pool. Replacing/clearing/removing the policy releases its cache when the
last native owner drops it. No callback registry or cross-runtime handle exists.

User capture slots remain omitted. The bounded backtracker is disabled to avoid
its text-length-dependent visited bitmap and stack; hybrid matching and PikeVM
fallback remain available. Admission reserves sixteen times the engine's reported
compiled size, the initial explicit cache size, four times the 256 KiB hybrid
capacity, source bytes and owner overhead. This allows for lazy active-state/
whole-match-slot/epsilon-stack storage, both hybrid caches and vector capacity
slack instead of charging only an initially empty cache. Each operation charges
that reservation against the existing 64 MiB tree/session payload budget before
commit; failed admission leaves the prior rule and charge intact.

This conservative accounting reserves at least roughly 1 MiB per filtered input,
so many simultaneously mounted filters can hit the budget before allocating that
much memory. It is not an allocator quota or a proven RSS/latency ceiling. Further
representative and adversarial measurements are required before release; do not
infer performance acceptance from compilation limits or one cache-growth test.

## Public gallery and remaining acceptance

The Editors page's "Keep typing on track" card switches between a workspace
handle, formatted reference and free text on one retained editor. Sample loading
uses revision-conditional replacement. Its immutable rules are prepared once from
application Eio initialization, before windows open, and passed to Bonsai; they
are not compiled during evaluation or on every render.

Local native and paired tests cover the implemented contract. Real keyboard/IME,
clipboard, external accessibility/VoiceOver, visual gallery and independently
installed-consumer runtime checks remain, alongside aggregate resource/performance
workloads and final required Linux checks. See the
[extension plan](plain-input-extensions.md) and
[validation evidence](../evidence/input-validation-och41.md).
