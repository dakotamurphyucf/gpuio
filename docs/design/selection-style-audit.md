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

## Remaining implementation and acceptance

- Document bodies still need to honor inherited/local `User_select false` in all
  source, code, diff and prepared-Markdown modes. Disabling the entire pointer
  subtree is not equivalent: links, file controls, explicit Copy source and scroll
  must remain usable. Disabling selection must clear obsolete selected ranges and
  drag state without replacing the source or losing unrelated control focus.
- The pinned Markdown widget has a selectable flag; the pinned source editor does
  not expose an equivalent user-selection policy. Define a narrow native contract
  for pointer, keyboard, accessibility and explicit commands, and validate it
  across mode changes and streamed replacement before advertising support.
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
