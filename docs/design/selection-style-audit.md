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

GPUIO currently keeps ordinary `View.text` selection per node. GPUIX also supports
window selection spanning multiple read-only runs. The existing GPUIO document
view supports selection inside its prepared document, but that does not establish
cross-node or cross-document selection parity. Resolve and validate this remaining
functional gap before closing the catalog audit; it is not silently deferred by
this document.

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
programmatic selection. These scopes do not turn ordinary per-node text selection
into cross-node selection; that adapter remains separate work.

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
`5abb243f7c6a27f75bd8fe3afcc77dd0a50e72c629cb26420fef42b78b726556`.
The patch adds the editor policy, Markdown disable cleanup and copy guard, public
native scope wrapper, and active-scope Copy/presence filtering. No public OCaml or
wire format changes are needed.

## Remaining implementation and acceptance

- Complete document selection policy evidence for disabled Markdown multi-click,
  scrolling, file controls and accessible selection/range commands. The focused
  native checks above are not the whole OCH-17 input/AX matrix.
- Ordinary cross-node and cross-document drag/copy behavior needs an explicit
  implementation/acceptance decision, including exclusions, virtual-row reuse,
  source changes, modal/window boundaries and bounded retention. Existing per-node
  or per-document evidence does not prove it.
- Complete inherited-style checks through deferred/specialized roots and the
  public/installed-consumer gallery matrix. Retained state and compilation alone
  are not all-root behavioral evidence.
- Complete the remaining style values, nested APIs and OCH-17 native, accessibility,
  performance/resource and distribution/release gates. Real Linux desktop
  qualification remains OCH-47; Linux build/unit/private-bus/consumer checks remain
  required for the current milestone.
