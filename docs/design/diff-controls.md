# Diff controls and source ownership

Status: OCH-41 implementation design. The bounded parser, visible-source
projection, Core configuration/event types, paired standalone codecs and native
control state exist. View configuration, live transport and stale-event routing
also exist. Mounted configuration now drives one projected native editor, with
selection/page/search and GPU highlight evidence. Show more also has native
pointer/keyboard/macOS accessibility and queued-event evidence. Rich line
activation now also has mounted pointer/Enter/AX evidence. Per-file headers have
native collapse controls, metadata and pointer/keyboard/AX checks. The public
gallery demonstrates managed/controlled expansion, streaming and queued events.
Broader gallery/consumer and release acceptance remain pending.
This document does not advertise a capability.

The reference is GPUIX at
[`18e695ed0ee8121a7793413ca795e08eda2a13df`](https://github.com/remorses/gpuix/tree/18e695ed0ee8121a7793413ca795e08eda2a13df):
`packages/native/src/diff/mod.rs` and `custom_elements/diff.rs`. Its controlled
`collapsedPaths`, `maxLines`, `onToggleFile`, `onShowMore`, and `onLineClick`
behaviors are the functionality target. GPUIO additionally supplies explicit
native-managed defaults. Hot reload is unrelated to this scope.

## OCaml interface direction

`Document.Diff` supplies domain modules. `Document.Config.create ?diff` is valid
only with `Mode.Diff`; omission preserves the existing raw presentation. Core and
Bonsai `View.document ?on_diff` now share the document node's handler router with
unchanged navigation. Supplying `on_diff` without explicit diff configuration is
a reconciliation error. Controlled collapse and preview settings now change the
mounted editor. Show more supports managed updates and controlled asynchronous
intent. Rich line observations are mounted; interactive file headers remain
unfinished.

The following constructors exist under `Document.Diff`:

```ocaml
module File_key : sig
  type t = Path of string | Unnamed
end

module Collapse : sig
  type t =
    | Managed of { initially_collapsed : File_key.t list }
    | Controlled of File_key.t list
end

module Line_limit : sig
  type t =
    | Managed of { initial : int option; step : int }
    | Controlled of int option
end

module Config : sig
  type t
  val create
    :  ?collapse:Collapse.t
    -> ?line_limit:Line_limit.t
    -> ?word_diff:bool
    -> unit
    -> t Or_error.t
end
```

Defaults are native-managed expanded files, no line limit, a200-line show-more
step and word emphasis enabled, preserving today's emphasis behavior. A limit is
0..8192; a step is1..8192. `None` means no preview limit. These limits do not
raise the separate256KiB/8192-line parser budget or suppress explicit fallback.
Path keys are nonempty, NUL-free UTF-8 labels at most4096 bytes. `Unnamed` is
explicit rather than an empty-string sentinel. Configs accept at most8192 unique
keys and256KiB of encoded configuration, validated on both sides of the bridge.
Unknown controlled keys are allowed so an application can name a streamed file
before its header arrives. Duplicate path sections share collapse state; events
also identify the exact source-local file index that was activated. Indexes are
snapshot positions, not durable application IDs.

Native-managed values initialize on mount, source replacement/generation reset,
entry into diff mode, or transition from controlled to managed ownership. Changing
an initial seed while already managed does not overwrite user interaction; keyed
remount/reset is the explicit reinitialization mechanism. A changed step affects
the next activation. Controlled values never change locally: activation reports
intent and waits for the application's next configuration. Switching to controlled
ownership immediately uses the supplied value.

Same-generation appends preserve managed state. Native override storage is bounded
by currently parsed file keys; removed keys are discarded when a new snapshot
installs. Seed keys remain bounded configuration and supply the initial value for
newly appearing files. Source replacement, reset, unmount and window close release
overrides and projection state. A whole-document collapse retains these values.
The native controller rejects decreasing generations within its source lifetime;
switching registered sources creates a new controller rather than reusing that
generation sequence. Controlled membership uses an indexed key set rather than
scanning every configured key for every file.

## Visible rows and source mapping

Retain one native read-only editor. Build its visible text from complete original
source-line slices and retain an explicit mapping for every displayed row. The
canonical registered `Text_source` never changes when controls change. This avoids
one editor per line/file and preserves the existing native input/selection engine.

- An expanded file includes its metadata, hunk headers and body. A collapsed file
  retains its first source header; following metadata/body rows are removed from
  the layout projection. The mounted adapter must supply the file label, status,
  counts and accessible collapse action at that header.
- `maxLines` counts context, added, removed and no-final-newline annotation rows
  globally across expanded files. Preamble, file and hunk headers do not count.
  Stop before another file/hunk header when the limit is exactly exhausted.
- Hidden-line count includes expanded body rows withheld by the limit. Collapsed
  bodies are counted separately and never inflate Show more. No actionable control
  is shown when the hidden count is zero. Managed Show more raises the limit by
  its configured step, capped at8192; controlled Show more reports the hidden count.
- Syntax runs and native hunk-fold candidates map through the same projection.
  A hidden hunk header cannot leave a fold candidate on a different visible row.
  Preparation/configuration builds mappings; layout and paint never call OCaml.
- Native selection copies the selected visible text. It cannot silently include
  collapsed rows crossed by a display selection. Copy source still copies the
  complete canonical source, including hidden and preview-limited content.
- A projection change preserves anchor/caret direction only when the exact selected
  source intervals and bytes remain one contiguous display interval. Otherwise it
  clears selection. A caret at a join belongs to the following visible source row;
  projection EOF belongs to its last visible row's end. Generation/continuity checks
  remain the mounted owner's responsibility.

Use the existing bounded source-page policy for large visible projections. Display
page offsets and canonical offsets must remain distinct. Page labels must identify
visible diff bytes when a projection is active. Searching a hidden canonical match
must offer the existing raw-source presentation rather than silently override a
controlled collapse/limit. Switching back restores the configured diff projection.
Raw-source fallback and raw-source selection remain explicit modes.

The subtree-highlight collector uses the installed visible page text with its exact
snapshot identity. Explicit ranges use that displayed text's coordinates, consistent
with other rendered text fragments. A projection change invalidates its match/paint
owner; ordinary native hunk folding still changes paint visibility without changing
installed-page logical counts. The shared parser/source resource pool now also
charges retained projection text, row maps and editor page buffers before mount.
The mounted adapter reserves additional conservative units from the same64MiB pool
before building a replacement; the old reservation stays live during replacement.
Controller accounting includes captured managed seeds even after a smaller config
replaces their original declaration. Admission failure visibly falls back to raw
source while retaining the last managed values and their charge. This is bounded
resource admission, not an exact allocator/RSS measurement.

## Event and bridge contract

The additive operation is `SetDocumentDiff` (tag58: node, positive epoch, optional
config), and the event is `DocumentDiffEvent` (tag65). Existing document config and
navigation packets retain their layout. Capability advertisement remains withheld
until mounted acceptance. Reconciliation generates a monotonic per-node epoch;
cosmetic updates and closure refreshes preserve it. Changing, clearing or restoring
diff configuration advances it, so earlier queued intents cannot become current
again. Native tree changes validate the epoch/mode atomically and charge retained
configuration bytes.

Events carry window/node/handler/tree identity, source resource, installed source
generation/revision and configuration epoch. Domain payloads distinguish:

- File toggle: file index, key, before/after labels, requested collapsed value and
  whether native-managed ownership applied it.
- Show more: current visible body count, hidden body count and the newly applied
  managed limit when applicable. Controlled mode supplies intent without mutation.
- Line activation: file index/key, optional one-based old/new coordinates, exact
  payload text and its canonical UTF-8 byte range. Context has both coordinates;
  additions/removals have one; annotations have neither. The event describes the
  installed snapshot even while a newer same-generation parse is pending.

The standalone schemas are `Document_diff_wire` and Rust `document_diff`.
`Event` contains positive `config_epoch`, `source_revision`, `source_generation`
and the observation; the transport envelope supplies the window/node/
handler/resource/tree identity. Line payloads are at most16KiB, valid UTF-8 without
LF, and exactly match the length of their bounded canonical byte interval. Empty
payloads, embedded NUL and a standalone CR remain valid source text. Paths have
the stricter NUL-free, nonempty label contract. File keys must agree with the
after-path (or before-path for deletions); unnamed files have neither path.

Bounded readers reject oversized strings/lists before allocating their contents.
Both standalone decoders reject truncation, trailing bytes and malformed values.
Configuration-aware validation additionally rejects controlled toggles that do
not invert the configured state, an applied mutation in controlled mode, a
show-more request for the wrong current limit, and managed increments that do
not match the configured step. This validation does not replace mounted epoch,
handler and source-registration fences.

Native handlers check the current presenter/config/source identities before queuing.
The OCaml registry rejects reset/released/wrong-generation resources; reconciliation
rejects obsolete handler/config epochs. Both source stores retain the first
published revision of a generation, bounding accepted old-picture revisions.
OCaml additionally accepts the exact pending Publish before its acknowledgement,
but never Begin/Chunk stages or future revisions. A desired reset retires old
events immediately, before its upload completes. Queue accounting includes every
path copy and line payload, preserves event order, and fits drains to the existing
packet limit. Source revision provenance is independent
of the tree revision. Payload limits match the parser and are validated before an
event reaches an application. Do not interpret snapshot-local byte ranges or file
indices against a newer source without checking/rebasing them.

Show-more activation captures the installed snapshot, displayed-page identity,
configuration epoch and handler binding. The native presenter rejects retired
pages/configs/callbacks, replaced presenters, hidden/modal-blocked nodes, reset or
released sources, and closed/overloaded windows before changing managed values.
Managed activation rebuilds locally and optionally queues an observation; no
application callback is required. Controlled activation only queues intent.
Removing the footer after a managed expansion or controlled config update repairs
keyboard focus to the current primary document control.

For rich line observations, a primary click must land on laid-out source text
with a collapsed selection; gutter/blank-area clicks and selection drags are not
line activation. Unmodified Enter activates the editor caret, including EOF on a
nonterminated final row. Headers do not emit line observations. Added, removed,
context and annotation rows retain their distinct coordinate/payload semantics.
The existing Go to line toolbar/AX action also sends the rich observation when
explicit diff settings are installed, while preserving legacy navigation where
a coordinate exists. Raw-source mode keeps legacy navigation without rich events.
Both callbacks can therefore run for one toolbar action when both are registered.

Pointer and keyboard/accessibility activation share the same native operation. No
synchronous callback, filesystem read, syntax acquisition or network access is
allowed from native rendering. File labels are display/navigation metadata only.

## Native file headers

Each fully installed first file row keeps its original selectable source text.
A native gutter button toggles that file; a suffix reports Added, Deleted, Changed
or Binary status and added/removed counts, plus the destination on a rename.
The label and expanded accessibility state belong to the button. Metadata is
decorative and never enters source selections, highlights or line payloads.

The pinned Base adapter supplies a bounded read-only, nonwrapping editor row hook.
It places gutter and suffix elements during the same prepaint as the shaped source,
using visible buffer-row mappings after folds and scrolling. The gutter shares
the fold slot, and suffix width contributes to horizontal scroll extent. Suffix
painting is clipped outside the gutter. Neither changes row height or creates a
separate editor for each file. No geometry from the preceding frame positions
the controls. The map admits at most1024 current-page rows and finite suffix widths
from0 to1024 logical pixels; GPUIO uses a240-pixel ellipsized metadata suffix.
Source limits and shared projection admission continue to bound retained data.

Header activation uses the same captured snapshot/page/epoch/handler checks as
Show more, plus membership in the current editor viewport's header map. Managed
collapse rebuilds locally; controlled collapse queues intent. Replacing the callback
binding refreshes retained header actions even when configuration is unchanged.
Same-generation files with the same index/path keep their focus handles across
projection updates. Tab traverses viewport headers before the source editor;
removing a focused header returns focus to the primary document control. Raw
source and non-diff modes clear the adornments.

## Syntax and word emphasis

Background preparation resolves each side's language from its own filename label,
without opening the file. Deleted lines use the old language, added lines the new
language, and context uses the new side (or the old side if only that language is
recognized). The two grammar states are independent, including on language-changing
renames. Synthetic side buffers omit diff markers and annotations; UTF-8 intervals
map tokens back onto exact canonical payload ranges. Source markers, line endings,
metadata colors, diff backgrounds and selection coordinates remain intact.

Each hunk starts a fresh syntax context. A patch cannot recover lexical state from
omitted source; carrying a comment/string through an unknown gap can miscolor later
hunks. No padding or array is indexed by original line numbers, so a distant hunk
does not inflate work. Unknown languages keep diff colors. Syntax errors or run
limits fall back to complete diff coloring while retaining controls; cancellation
remains cancellation. Admission includes side buffers, parser metadata, row maps,
and intermediate/final run vectors under the shared work budget. Highlighting stays
off layout/paint and capped at the existing limit of 32,768 runs.

Word emphasis pairs equal-length contiguous removal/addition groups by line index
within each hunk. Unequal groups retain ordinary diff colors rather than guessing
which lines correspond. Common prefix/suffix comparison uses Unicode alphanumeric
or underscore word tokens, with punctuation/whitespace as individual tokens. It is
linear trimming, not full LCS. Emphasis is separate from syntax font flags:
`word_diff=false` removes only change emphasis, preserving grammar styling.

The public Documents gallery's Diff tab uses a small OCaml reducer for controlled
requests; managed observations leave its native initialization seeds unchanged.
It shows multiple file languages, bounded preview/Show more, file and line events,
streamed file additions and source-generation reset. The gallery can also be built
against a staged installation without changing any opam switch. See its README
and the focused evidence before treating the larger catalog as accepted.

## Acceptance still required

The projection unit tests cover bytes, hidden gaps, selection transfer, syntax and
hunk mapping, streaming prefixes and exact limits. Paired config/event fixtures,
validation and native state-transition tests now also pass, including controlled
nonmutation, managed seed retention/reset and pruning of removed-file overrides.
Core/Bonsai routing, live-envelope fixtures, tree/queue validation and source
registry tests also pass. Mounted config updates now pass retained-editor,
selection/direction, canonical navigation/search, bounded-page and source-revision
checks. Native GPU checks cover projected matches, selection precedence, hunk
folding and retired painter disposal. Per-file pointer, Tab/Space/Enter and macOS
AX actions now pass, including managed/controlled ownership, handler refresh,
scrolling and focus cleanup. Focused public gallery and independently installed
consumer checks now pass for managed/controlled expansion, word configuration,
typed line actions, streaming, theme/size retention and reset. Syntax and word
group tests pass separately. Completion still requires broader header horizontal
clipping, source/page/search/copy and streaming/selection behavior, bounded
application resource/performance tests, the combined gallery/consumer matrix and
required release gates. These checks do not establish all GPUIX behavior parity.
