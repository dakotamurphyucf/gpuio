# Diff controls and source ownership

Status: OCH-41 implementation design. The bounded file parser and visible-source
projection exist; the public controls, protocol routing and mounted presentation
described below are still pending. This document does not advertise a capability.

The reference is GPUIX at
[`18e695ed0ee8121a7793413ca795e08eda2a13df`](https://github.com/remorses/gpuix/tree/18e695ed0ee8121a7793413ca795e08eda2a13df):
`packages/native/src/diff/mod.rs` and `custom_elements/diff.rs`. Its controlled
`collapsedPaths`, `maxLines`, `onToggleFile`, `onShowMore`, and `onLineClick`
behaviors are the functionality target. GPUIO additionally supplies explicit
native-managed defaults. Hot reload is unrelated to this scope.

## OCaml interface direction

Add `Document.Diff` domain modules and an optional `diff` argument on
`Document.Config.create`, valid only with `Mode.Diff`. Keep navigation unchanged;
add a separate optional `on_diff` callback to Core and Bonsai `View.document`.
Both callbacks share the document node's handler router.

The intended shapes below are a design sketch, not currently callable signatures:

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
installed-page logical counts. The existing parser/source resource reservation must
also charge retained projection text, row maps and editor page buffers before mount.

## Event and bridge contract

Introduce an additive document-diff configuration operation and typed event; do not
change the existing document configuration packet or navigation event layout.
Negotiate/advertise the new capability only after mounted acceptance. Generate a
monotonic configuration epoch in reconciliation, internal to the API. Restyles that
leave diff configuration unchanged preserve it. A changed configuration invalidates
older queued diff intents, independently of the source generation and tree revision.

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

Native handlers check the current presenter/config/source identities before queuing.
The OCaml registry rejects reset/released/wrong-generation resources; reconciliation
rejects obsolete handler/config epochs. Source revision provenance is independent
of the tree revision. Payload limits match the parser and are validated before an
event reaches an application. Do not interpret snapshot-local byte ranges or file
indices against a newer source without checking/rebasing them.

Pointer and keyboard/accessibility activation share the same native operation. No
synchronous callback, filesystem read, syntax acquisition or network access is
allowed from native rendering. File labels are display/navigation metadata only.

## Acceptance still required

The projection unit tests cover bytes, hidden gaps, selection transfer, syntax and
hunk mapping, streaming prefixes and exact limits. They do not prove mounted UI.
Completion requires paired OCaml/Rust config/event fixtures and validation; Core/
Bonsai reconciliation and resource-generation tests; native per-file/show-more
keyboard, AX and pointer controls; source/page/search/copy behavior; selection and
GPU highlight mapping under streaming; bounded lifetime/resource tests; the public
gallery and independent consumer; and the required release gates. Audit word-diff
options and path-based syntax behavior separately before claiming GPUIX parity.
