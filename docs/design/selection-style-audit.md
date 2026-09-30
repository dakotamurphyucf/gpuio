# Selection styles: inheritance and remaining parity work

Status: OCH-41 audit, not complete selection acceptance. This records current
behavior and remaining work against GPUIX at
`18e695ed0ee8121a7793413ca795e08eda2a13df`. The source references below were read
from that pinned checkout, alongside the committed TypeScript catalog snapshot.

## Values and defaults

GPUIX `renderer.rs`'s `Inherited::root`, `Inherited::descend` and
`selection_start_flag` establish these rules:

| GPUIX declaration | GPUIO ordinary-text expression | Meaning |
| --- | --- | --- |
| `userSelect: "text"` | `Style.Property.User_select true` | Enable, including beneath a disabled ancestor. |
| `userSelect: "auto"` | `Style.Property.User_select true` | The pinned implementation enables selection; it is not an inherit/reset value. |
| `userSelect: "none"` | `Style.Property.User_select false` | Disable in this subtree, unless a child explicitly enables it. |
| Omitted | Omit the property, or `Style.unset style User_select` | Inherit the nearest remaining ancestor declaration. |

GPUIO's documented ordinary-text default remains **opt-in**; GPUIX's root default
is enabled. An application can enable ordinary text at its root with
`User_select true`. This is a deliberate default difference, not an assertion of
identical CSS/browser behavior. Native editor inputs keep their widget-owned
selection and editing semantics; their selection must not be disabled simply
because surrounding application chrome has `User_select false`.

Ordinary `View.text` nodes now participate in Base's window selection engine,
sharing native paint order and focus-trap scopes with Markdown participants.
Single-click drags can span ordinary nodes, while keyboard ranges and double/
triple-click gestures remain local to the focused node. This is a cross-node
foundation, not complete cross-document acceptance. Mixed ordinary/Markdown
native checks are recorded below, alongside ordinary-window isolation and managed
endpoint eviction. Multiple documents, larger virtualized ranges and the broader
input matrix remain required before closing the catalog audit.

## Selection color implementation

`Selection_color` is inherited independently of whether a text node currently
accepts selection. A local declaration wins over an ancestor; the last local
base declaration wins. Clearing the local declaration restores inheritance, and
clearing the final ancestor declaration restores the receiving widget's native
default. Colors are resolved from the public `Color`/theme interface before the
native style bridge.

Native `Interaction` now carries `Option<Hsla>` for this field. `None` denotes
absence of an explicit override, rather than assigning one shared hard-coded
color to every native widget. Ordinary text keeps its original default selection
color. Lists, table child views, toasts and tooltip/hover-card content retain this
optional value while passing inherited interaction state to descendants.

Previously, document bodies ignored both inherited and local `Selection_color`.
The document presenter now resolves the last local declaration and then the
inherited value, and applies it to its source editor and Markdown text style.
Updates preserve the native widgets, source snapshot and selected text. A prepared
source/page replacement reapplies the stored override. No wire, public constructor,
parser or vendored fork change is needed.

## Document user-selection policy

Document source, code, diff and prepared Markdown bodies default to selectable.
An explicit `User_select false` on the document or an ancestor disables user text
selection; a local `true` restores it, and removing that local declaration restores
inheritance. Internally `Interaction.selectable` carries an optional declaration,
so the ordinary-text opt-in default and native-document default remain distinct.
This does not change the public Boolean property or wire representation.

The pinned Base editor now has `set_user_selectable(bool, cx)`, enabled by default.
Disabling it collapses the current range at its head, removes extra cursors and
cancels drag/autoscroll state. Pointer clicks can still place the navigation caret,
but dragging, multi-click selection, Shift range extension, user Select All and
selection Copy do not select or copy text. Programmatic bridge selection remains
available for explicit search/navigation. The selection Copy command remains
unavailable even for such a programmatically created range. This adapter control
is applied to read-only document editors; editable input/textarea/editor widgets
retain their own selection and clipboard semantics.

Markdown disabling resets local selection and drag state, stops autoscroll and
omits the window-selection registration during paint. Its copy projection rejects
a disabled view even before the previous registration is swept. Selection policy
does not block the pointer subtree: document links, toolbar buttons, explicit
Copy source/code/table actions, diff controls and scroll remain separate controls.
This is an interaction policy, not content protection: source text stays available
through the document API and accessibility. Broader accessible selection/range
acceptance remains in OCH-17.

The GPUIO window now mounts Base's `TextSelectionLayer` as its first child. The
native pointer regression exposed its previous absence: programmatic select-all
could paint a range, but Markdown drag/copy lacked a window selection owner. Each
retained focus trap receives a stable selection-scope identity; documents register
in the containing trap, and the host activates the current top trap. Changing the
active trap clears obsolete selection. Both Copy and selection-presence queries
exclude participant-local selections from inactive scopes, including a stale
programmatic selection. Ordinary text now registers in the same scopes and paint
order through the adapter described below.

Style updates keep the native editor/Markdown entity and installed source snapshot.
The native default stays enabled when all declarations are removed, while a
previously cleared selection is not intentionally restored.

## Local evidence

`document_selection_style_test.rs`, included in `native_highlight_document`,
passes actual GPU selection-color checks for code/source and Markdown:

- Inherited opaque magenta reaches the selected glyph background.
- Multiple local declarations resolve to the last value, opaque cyan.
- Changing the ancestor to green preserves the local cyan override.
- Removing the local override restores inherited green; removing the ancestor
  override removes that green and restores the native default.
- Restyling retains source-snapshot identity, native editor/Markdown entities and
  the selected text or exact source selection byte range.
- A same-generation streamed append keeps the chosen selection color in actual
  GPU pixels, retains the native widgets and preserves the previous selected
  text/range rather than extending selection into the new text.

The existing document GPU suite also passes source scrolling, state styling,
diff folding/projection, selection precedence and painter disposal. `native_ui`
passes its existing ordinary-text selection/copy, focus, pointer-policy and style
reset checks. Production compilation, strict all-target native/protocol Clippy
and formatting pass.

Commands on macOS14.5 arm64:

```sh
./scripts/gpuio exec cargo check -p gpuio-native --locked -j2
./scripts/gpuio exec cargo check -p gpuio-native --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --test native_highlight_document --test native_ui --features native-image-tests --locked -j2 --no-run
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
./scripts/gpuio exec cargo fmt --all --check
git diff --check
```

Both Cargo-reported native executables ran sequentially under90-second process
watchdogs and reached their final success markers. The initial test helper needed
the correct Markdown focus accessor and a cloned handle to end the immutable
borrow before focusing. The streaming fixture initially tried to reopen a Complete
source as Streaming in the same generation, which admission correctly rejected;
starting that fixture as Streaming fixed its lifecycle. The reset helper also
now increments the actual source generation independently of revision; appending
advances revision without advancing generation. These were fixture fixes.
No physical keyboard/IME, screen-reader or Linux desktop acceptance is inferred.

## Document policy and window-selection evidence

`document_selection_policy_test.rs` now runs inside the same native document
executable. Local macOS 14.5 arm64 results cover code, diff and Markdown:

- Default selection, inherited disable, child re-enable, local unset and ancestor
  unset; source and native entity identities survive style-only changes.
- Keyboard Select All/range extension/Copy rejection while disabled; source
  single/double/triple clicks and dragging do not create a range.
- Re-enabling restores actual native selection Copy. Code/source Copy preserves
  source bytes including the trailing newline; the pinned Markdown Copy action
  currently trims its rendered-text result. That existing behavior is not a
  byte-exact source-copy contract; the explicit Copy source action is separate.
- A real native mouse drag first selects text, disabling during that drag clears
  it, and re-enabling does not restore the retired range. This includes Markdown
  through the newly mounted window selection engine.
- Programmatic source selection stays available while selection Copy is disabled.
  The keyboard-operated Copy source button still copies the full source, and a
  disabled-selection Markdown link still sends its queued navigation event.
- A same-generation streamed append keeps selection disabled and retains the
  native source/Markdown entities.
- Opening a focus trap clears background Markdown selection. A deliberately
  stale programmatic background select-all cannot enter the active scope's Copy
  result or selection-presence query; closing the trap does not resurrect it.

The broader `native_ui`, `native_editor` and `native_controls` executables also
pass. The native editor fixture now applies `User_select false` to its parent,
while its existing grapheme, clipboard, focus, revision, auto-grow, AppKit marked/
committed text, undo and accessibility checks remain enabled. These are actual
native windows and GPUI/AppKit test dispatch; they do not establish physical IME
or screen-reader acceptance.

Build the four executables with:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --test native_highlight_document --test native_ui --test native_editor --test native_controls --features native-image-tests --locked -j2 --no-run
```

Each Cargo-reported executable ran sequentially under a 90-second process-group
watchdog and reached its final success markers. The initial drag test exposed the
missing window selection layer, which was fixed in the host. A later expected
clipboard fixture incorrectly trimmed source Copy and was corrected to preserve
source bytes; no production source-copy change was needed.

The Base patch is reconstructed from the unchanged pinned archive by
`scripts/vendor_gpui_base.py`; the resulting tree matches `vendor/gpui-base`
(excluding generated `Cargo.lock`). `third_party/sources.json` records patch SHA256
`61e91aef2a2c46bf6db7aa60b7dd261e33c8b053e57988938467c6699b46eee6`.
The patch adds the editor policy, Markdown disable cleanup and copy guard, public
native scope wrapper, active-scope Copy/presence filtering, ordinary participant
paint-order/local-anchor APIs, whitespace Copy preservation and retired-endpoint
cleanup. Markdown copy normalization is participant-local, and its Select All
action retires prior shared geometry outside the widget borrow. Ordinary run
projection also uses cached shaped glyph cells and extended-grapheme boundaries,
with current alignment applied at projection time. No public OCaml or wire format
changes are needed.

## Ordinary window-selection adapter

Each retained ordinary text node owns one Base participant. Its callbacks hold a
weak reference to the node's native state; the window registry cannot keep an
unmounted node alive. Paint registers current shaped geometry and the containing
focus-trap scope. Pointer projection, local keyboard commands and Copy remain
entirely native; no per-frame selection payload crosses the OCaml bridge.

Single-click drag and Shift-click span eligible participants. Copy collects them
in rendered order, joining nonempty results with a newline and preserving their
selected source bytes, including whitespace-only selections. `User_select false`
excludes ordinary nodes. A local keyboard range retires prior shared selection;
Cmd/Ctrl+A remains focused-node Select All. Editable widgets retain their own
selection engine. Native source changes retire geometric selection before the
new source is exposed; local ranges clamp to valid UTF-8 boundaries.

A shared source/display mapper now serves both search washes and selection.
Truncated pointer ranges select retained source glyphs, never synthetic ellipsis
bytes. A range crossing both retained pieces of middle truncation includes the
intervening source bytes; an ellipsis-only geometric range is empty. Keyboard
Select All includes hidden source. Hidden caret positions snap to the nearest
retained edge for Shift-click anchoring. Public clip, end-ellipsis and start-
ellipsis styles retain their existing wire values; no middle-ellipsis public
constructor is added by this change.

Selection painting follows projection in the same frame, after search washes and
before glyphs. Reordering uses the endpoints' retained participant identities and
new painted positions. When an endpoint stops registering for a completed frame,
the window retires the gesture and stops autoscroll before dropping the endpoint.
Showing that retained participant again cannot resurrect the old selection.
Interior participant retirement alone does not discard surviving endpoints.

The native `selection_window_test.rs` fixture runs inside `native_ui` and covers
forward/reverse Unicode drag/Copy, exclusions, source replacement, disabling an
interior participant, local grapheme movement, keyboard-to-Shift-click extension,
exact whitespace Copy, actual start/end ellipsis shaping and reverse drag,
full-source Select All, reorder, hide/show endpoint retirement and modal clearing.
These use actual native windows/layout/clipboard with GPUI-dispatched input; they
are not physical keyboard/IME or VoiceOver acceptance. Pure mapper tests also
cover Unicode middle/affix-only and equal-byte-length truncation; those tests do
not establish mounted middle-truncation behavior.

The retirement regression first reproduced stale selection returning after an
endpoint was hidden and shown. Base's post-frame sweep now retires endpoint state
as well as clearing participant ranges. This is separate from source replacement
and active-modal scope retirement.

Local macOS validation of this adapter passed with observed exit 0 for
`native_ui`, `native_highlight_view`, `native_highlight_document` and `native_list`.
The last test completed two 100,000-row traversals, retained at most 256 active
views/selection caches, and released evicted row payloads. This preserves the
existing list regression; it is not acceptance of every virtualized cross-node
selection gesture. Document selection-color/policy, streaming, modal isolation
and native gutter/source/highlight checks also passed after the shared Base change.

```sh
./scripts/gpuio exec cargo test -p gpuio-native --lib text_projection --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --lib highlight_paint::tests --features native-image-tests --locked -j2
./scripts/gpuio exec cargo test -p gpuio-native --test native_ui --test native_highlight_view --test native_list --test native_highlight_document --features native-image-tests --locked -j2 --no-run
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
./scripts/gpuio exec cargo check -p gpuio-native --locked -j2
./scripts/gpuio exec cargo fmt --all --check
python3 scripts/audit_component_catalog.py
git diff --check
```

The Cargo-reported executables ran individually in process groups, with 90-second
watchdogs for UI/highlight/document and 300 seconds for the full list traversal.
Five shaped-paint unit tests and the source/display mapper test passed. Fork
reconstruction matched the committed snapshot excluding generated `Cargo.lock`.
The catalog check establishes structural coverage only (three reviewed style value
sets); no new capability advertisement or hosted release acceptance is implied.

## Mixed ordinary text and Markdown

Window Copy now produces the same result regardless of whether ordinary text or
Markdown owns keyboard focus. Markdown keeps its existing rendered-copy edge
trimming within its participant callback; it cannot trim neighboring ordinary
text. The aggregate inserts one newline between nonempty participant results.
This also avoids adding both a Markdown trailing paragraph separator and a second
aggregate separator at that boundary. Ordinary text still preserves selected
source whitespace. This is rendered selection Copy; explicit Copy source remains
independent and preserves the original document source.

The native mixed fixture puts formatted Markdown between ordinary text nodes with
leading/trailing spaces and Unicode. It checks both drag directions, switches
keyboard focus between participant types without changing the selected range,
and begins a drag inside Markdown. Full source replacement retires the mixed
geometric range before a fresh gesture can copy the new installed document.
Removing the ordinary endpoint nodes clears the remaining shared selection.

User Select All in Markdown replaces the previous mixed range with that document's
local selection. The action clears window geometry before invoking the existing
state listener; doing so while the listener held the document's mutable borrow
would let clear callbacks re-enter the same entity. Disabled Select All retains
its existing propagation behavior. Programmatic selection APIs remain distinct.

`document_mixed_selection_test.rs` runs inside `native_highlight_document`.
Its Select All regression initially copied the previous neighboring text; the
native action repair fixes that state transition. These focused mixed checks do
not establish separate-window, all virtualized content, physical input or VoiceOver
acceptance.

On the final mixed-selection revision, both `native_highlight_document` and
`native_ui` exited 0 under separate 90-second process-group watchdogs. The document
suite includes existing code/diff/Markdown GPU, selection-policy, streaming,
focus-scope and source-retention regressions in addition to the mixed fixture.
Strict all-target native/protocol Clippy, default production compilation, formatting,
structural catalog and exact Base fork reconstruction also passed. The preceding
100k list evidence belongs to the ordinary-window-selection checkpoint above;
this narrower Markdown action change did not rerun that traversal.

## Separate windows and managed endpoint eviction

`selection_isolation_test.rs`, included by `native_ui`, opens a second native
window without requesting activation. It reuses node IDs while keeping selected
text, clearing and source-replacement effects independent between windows. Closing
the second window releases its native selection owners; the first keeps its range.
Reopening the same protocol window slot with a new generation starts with no
selection. The fixture checks these transitions for two successive generations,
including actual clipboard results routed to each window. It uses GPUI-dispatched
input; it does not assert physical active-window keyboard behavior or a broad
window-cycle memory budget.

`list_selection_test.rs`, included by `native_list`, selects across two materialized
rows. The focused anchor remains in native admission pins. The unfocused endpoint
can be legally evicted while its logical row remains in the source order, matching
the accepted managed-list policy that historical unfocused selection does not
pin resources indefinitely. Eviction releases the old selection owner and clears
the shared result rather than allowing a partial stale Copy. Rematerializing that
logical row under a new node generation does not restore the old range; a fresh
gesture copies the new payload. The surrounding native suite continues to cover
held-drag retention, source deletion, disposal and bounded 100k-row traversal.

These cases do not promise Copy over arbitrary unloaded history. The focused
interior-row case below complements endpoint eviction; broader combinations remain. An application's loaded logical data and bounded native
row materialization remain separate owners; see [managed lists](managed-lists.md).
They also do not establish mixed-document selection across independent windows.

Both `native_ui` and `native_list` exited 0 locally on this fixture revision, under
90- and 300-second process-group watchdogs respectively. The list suite completed
two 100,000-row traversals with at most 256 active native views/selection caches,
released evicted owners and completed its demand-convergence checks. Strict
all-target native/protocol Clippy with `native-image-tests`, formatting, diff and
structural catalog checks passed. This checkpoint adds tests and documentation;
it does not change production code, wire/API contracts or the pinned fork.

### Evicting an interior row

`list_selection_interior_test.rs` selects across three materialized rows, then
removes only the unfocused middle row through native guarded admission. The list's
logical order remains the same object: this is materialization eviction, not
source deletion. Its native selection owner is released. The two endpoint rows
remain selected and Copy includes their current text, without retaining the evicted
middle payload.

Rematerializing that logical middle row under a fresh node generation lets its new
payload join the still-live geometric range. This differs from rematerializing an
endpoint: an evicted endpoint retires the gesture, so there is no live range for it
to rejoin. Changing the selected interior row's source text retires shared geometry;
a fresh gesture then copies the current three-row text. These checks enforce
bounded native ownership rather than pinning every historically selected row or
copying arbitrary unloaded application data.

On this fixture revision, `native_list` exited 0 under a 300-second process-group
watchdog. It completed both 100,000-row traversals with at most 256 active native
views/selection caches, released evicted resources and passed demand convergence.
Strict native/protocol all-target Clippy with `native-image-tests`, formatting,
diff and structural catalog checks passed. This checkpoint adds tests and
clarifies the existing bounded selection contract; it makes no production,
protocol, dependency or fork changes. The traversal is not a full application
performance budget or physical-input/accessibility qualification.

## Ordinary pointer typography

Pointer range projection uses shaped glyph cells instead of estimating widths
from consecutive Unicode scalar caret positions. The latter copied only the first
person from a joined family emoji in the initial native regression. Each cell now
covers its entire extended grapheme; glyph source indices and visual positions
also preserve source-byte ranges when Hebrew/Arabic glyphs are reordered. A visual
selection projects to one contiguous logical source range. This does not introduce
multiple disjoint bidi selection ranges.

The native run lazily caches immutable cell geometry against source text and the
shaped line identities. Retained runs reuse it until text or shaping changes;
current paint bounds, line height and alignment apply when projecting. The cache
retains shaped line data, not native entities or OCaml callbacks. No additional
per-frame bridge traffic is needed. Hard-line separator cells preserve source
newlines, including CRLF as one extended grapheme.

`selection_typography_test.rs`, included by `native_ui`, checks actual laid-out
glyph bands independently of logical caret lookup. Forward and reverse drags copy
whole family emoji, combining-accent letters and flags, plus whole and partial
Hebrew/Arabic spans. All cases run with left, center and right alignment; the
latter two assert a nonzero layout inset. Additional cases select soft-wrapped,
LF and CRLF text in both directions and a CRLF separator alone. The final native
UI run exited 0 locally. These are native layout and GPUI-dispatched pointer/
clipboard checks, not physical input or full international typography acceptance.

The document, highlight and list regression executables also exited 0 on this
geometry revision, with sequential 90/90/300-second process-group watchdogs.
The list suite completed both 100,000-row traversals, retained at most 256 active
views/selection caches and released evicted resources. Strict native/protocol
all-target Clippy, production compilation, formatting, structural catalog checks
and exact Base fork reconstruction passed. The build commands and watchdog
procedure are the same as the ordinary-window adapter checks above; these results
do not replace application-scale selection cost or hosted release validation.

### Local caret and word selection

Ordinary pointer handlers and keyboard-to-Shift-click anchors now use that same
shaped geometry. Word hits resolve the source cluster under the pointer; single
clicks and local drags resolve its nearest caret edge. Each cell uses its resolved
Unicode bidi level to distinguish leading and trailing visual edges. The existing
`unicode-bidi` 0.3.18 dependency is now also a direct Base dependency; no package
versions changed. Adjacent glyphs painting one grapheme share one caret midpoint.

Logical source boundaries prefer the downstream cluster at bidi and wrap edges.
An index inside a shaped cluster snaps to its leading edge; this is not a promise
of platform-specific ligature caret subdivisions or two simultaneous bidi carets.
Read-only Left/Right movement retains logical extended-grapheme order. Editable
inputs retain their own native widget engine. The shared Base multi-click helper
also computes visual endpoints from selected glyph cells instead of assuming
logical offsets increase from left to right.

The added native regression first reproduced a centered accented-letter double
click selecting nothing. It now passes centered/right/left word selection for
accented, Hebrew and Arabic text; near-leading-edge clicks followed by Shift-Right
copy the intended complete grapheme, including joined emoji and flags. Keyboard
movement to a word start followed by Shift-click copies the expected span across
all three alignments and both text directions. Existing ordinary cross-node,
window-isolation, truncation and full wrapped-copy checks still pass. These are
actual laid-out glyph coordinates and native clipboard results with dispatched
GPUI input. Broader mixed-direction wrapping, multi-click cross-node semantics,
physical input and large-text projection/copy performance remain open.

On this caret revision, the 45 standalone Base `text_selection::tests` passed
against GPUIO's local GPUI/accessibility/layout patches. This includes the shared
multi-click, scope, retirement and separate-window state-machine tests; their GPUI
test context is distinct from the native UI evidence above. The application native
UI and document regression executables both exited 0 under separate 90-second
process-group watchdogs. Strict all-target native/protocol Clippy passed, as did production
compilation, formatting, structural catalog validation and exact Base patch
reconstruction. The two 100k-row traversals reported in the preceding section
belong to the prior range-projection revision, not a new performance measurement
of the bidi/caret path.

## Multiple rendered documents and pointer Copy controls

`document_multi_selection_test.rs` mounts two independently prepared Markdown
sources in the same window. Forward and reverse drags copy their rendered text in
paint order, with the same result when either document owns keyboard focus. Select
All remains local to the focused document. Reordering retains both Markdown
entities; a fresh drag follows the new rendered order. Replacing a selected source
retires the shared range, and a fresh gesture uses only the installed replacement.

Removing a selected document clears the surviving range and releases both its
presentation and Markdown owner. Reusing its protocol slot with a new generation
starts without selection; a fresh drag copies the new mounted document, and its
subsequent removal releases its presentation too. These checks cover two rendered
Markdown documents. They do not establish all mixed source-editor/document modes,
independent-window document combinations or virtualized interior-row eviction.

The selection-policy fixture also rejects Markdown single-, double- and triple-
click drags under inherited disable, preserving an empty selection and unchanged
clipboard. The existing keyboard link and toolbar paths remain usable.

The macOS `document_copy_control_test.rs` locates each native AXButton by label,
requires a unique nonempty screen frame, converts that frame to view coordinates
and asserts that its center is inside the viewport. It dispatches pointer
move/down/up at that point rather than invoking an accessibility action. With
`User_select false`, pointer Copy source preserves exact Unicode and newline bytes
in Markdown, code and diff modes. Markdown Copy code returns the fenced code
payload, and Copy table returns the renderer's normalized table Markdown.

This establishes the pointer activation path and native AX button labels/frames
for those controls. It is not a physical mouse/clipboard-shortcut or VoiceOver
workflow test, and it does not establish the complete table accessibility model.
The outer fixture restores the clipboard and closes its window. No production
code, public API, protocol, dependencies or fork changes are required by this
acceptance checkpoint.

The final `native_highlight_document` executable exited 0 locally under a
90-second process-group watchdog, including the existing GPU, source/diff,
streaming, selection-policy, modal and resource-release checks. Strict all-target
native/protocol Clippy with `native-image-tests`, formatting, diff and structural
catalog checks passed. Build the executable with:

```sh
./scripts/gpuio exec cargo test -p gpuio-native --test native_highlight_document --features native-image-tests --locked -j2 --no-run
```

This is focused local macOS evidence; hosted gates, full document accessibility,
application performance/resource budgets and release acceptance remain open.

## Remaining implementation and acceptance

- Complete document selection policy evidence for scrolling, remaining file controls
  and accessible selection/range commands. Disabled Markdown multi-click now has
  native evidence above. These focused checks are not the whole OCH-17 input/AX
  matrix.
- Complete broader mixed-document/source-editor and independent-window combinations,
  broader virtualized lifecycle cases, mixed-direction wrapping and caret/word
  combinations, and multi-click cross-node semantics. Two rendered Markdown
  documents, ordinary two-window isolation, managed endpoint/interior eviction/reuse and
  measured macOS pointer Copy controls are covered above.
  The focused ordinary-node regression does not prove those combinations.
- Measure projection and copy cost for large selected text and bounded retention
  during repeated mount/unmount/window cycles.
- Complete inherited-style checks through deferred/specialized roots and the
  public/installed-consumer gallery matrix. Retained state and compilation alone
  are not all-root behavioral evidence.
- Complete the remaining style values, nested APIs and OCH-17 native, accessibility,
  performance/resource and distribution/release gates. Real Linux desktop
  qualification remains OCH-47; Linux build/unit/private-bus/consumer checks remain
  required for the current milestone.
