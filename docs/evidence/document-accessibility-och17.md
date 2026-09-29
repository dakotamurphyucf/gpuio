# Document accessibility — OCH-17

Partial release finding repair, 2026-09-28, macOS 14.5 (23F79), arm64.
The [contract](../design/document-accessibility.md) describes the full intended
behavior. This checkpoint does not complete document accessibility or OCH-17.

## Heading levels and table structure — 2026-09-29

The macOS adapter now exposes the parsed heading level as numeric AXValue; the
heading's actual painted children continue to provide its text. This follows
[WebKit's macOS heading value mapping](https://chromium.googlesource.com/external/Webkit/+/b4170928e42cb313b7c8304a796879ddb2ff7f12/Source/WebCore/accessibility/mac/WebAccessibilityObjectWrapperMac.mm).
GPUI Base's wrapping and horizontal-scroll Markdown table renderers now declare
table/row/header/cell roles, row/column counts and zero-based indices. Rows use
distinct identities instead of sharing `"row"` within one table. Action controls
stay outside the table's data hierarchy; no editing or selection action is added.

The focused external macOS gallery test passes heading level 1, exactly one
three-row/two-column table, AXRows enumeration, distinct row and cell references,
row/column index ranges and ordered Unicode cell text. It repeats these checks
after collapse/expand and across three page departures/remounts. Native scrolling
reveals appended code before querying its control; merely waiting for an offscreen
virtual block does not materialize it. Existing link keyboard/AX activation,
streaming, read-only editor and diff controls pass in the same walkthrough.
The complete 23-section gallery also passes with these changes, including the
extension lifetime and transfer identity assertions and normal window shutdown.

Local checks passed with the pinned environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @fmt @test/gallery/runtest -j2
python3 scripts/test_gallery.py --section documents --images scratch/document-structure-images
python3 scripts/test_gallery.py --section all --images scratch/document-structure-combined-images
./scripts/gpuio exec cargo test -p gpuio-native --test native_document --features native-image-tests --locked -j2 --no-run
# Run the emitted native_document executable under a bounded process-group watchdog.
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
./scripts/gpuio exec cargo fmt --all --check
python3 scripts/audit_component_catalog.py
```

The emitted native document executable exits zero, covering Markdown tables,
selection/streaming, source/diff behavior and lease teardown. Both modified vendor
sources reconstruct exactly from pinned archives and reviewed patches. No version
changes are involved. Local logs use `document-structure` in the implementing
agent's ignored notepad directory. Initial failed attempts caught offscreen-control
assumptions and an incomplete ctypes wheel-event signature; the corrected tests
retain the original interaction assertions and reap their children.

The gallery exercises the default wrapping table layout. Horizontal-scroll table
AX behavior, all heading levels, header associations/column navigation, full
selected-text/range APIs, rich/image links and actual VoiceOver reading remain
separate acceptance work. These results do not establish complete table or
screen-reader accessibility, Linux desktop behavior or release resource budgets.

## Earlier body and read-only source checkpoint

The native source editor now exposes its installed page as a labelled read-only
multiline text input. Focus validates the current presentation and modal/visibility
scope. There is no value-replacement action. Markdown's custom Inline element now
exposes its actual painted text; heading, paragraph, list and list-item containers
carry semantic roles. The original omission came from painting StyledText directly
without forwarding its accessibility metadata, not from missing parsed content.

The macOS adapter also needed two narrow corrections: Heading used the literal
role `Heading` instead of `AXHeading`, and AppKit's external AXValue-settable query
reported true despite the existing selector predicate returning false for the
read-only document. `document-semantics.patch` fixes the role and forwards that
attribute query to the same capability predicate. Other attribute queries retain
AppKit's behavior. No dependency versions changed.

Local passing checks:

```sh
./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_gallery.py --section documents
python3 scripts/test_gallery.py --section all
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_document
./scripts/gpuio exec cargo fmt --all --check
./scripts/gpuio exec cargo clippy --locked -p gpuio-native --features native-tests --all-targets -- -D warnings
```

The external macOS AX test reads rendered heading/body/list/link-label text,
including CJK and a joined family emoji; observes native heading/list roles;
appends and resets a streamed heading; reads code and diff values; verifies the
code value is not settable; focuses it and confirms Command-A followed by
Backspace and typing preserves the source. Collapse removes the diff text area,
expand restores it, and repeated page departure/remount still passes.
The combined twelve-section regression passes as well, including editable-field
AXValue replacement, overlays, managed collections and file-picker cancellation.

The native document regression also passes code, Markdown/table/fence/safe-image,
diff, Unicode streaming/selection and lease teardown. Its measured layout/paint
work is diagnostic, not physical presentation or a release performance threshold.
Both modified vendor trees reconstruct from their pinned upstream archives and
ordered patches; source comparison excludes GPUI Base's generated Cargo.lock.
Failed intermediate probes remain local notes, not acceptance evidence.

Remaining release requirements include rich-fragment/image link accessibility,
complete table/header reading semantics, native selected
text/range APIs, copy, modal/stale-action cases, bounded source pagination through
assistive tools, and actual VoiceOver reading/navigation. No hidden duplicate raw source substitutes for these
requirements. Linux desktop accessibility remains deferred to OCH-47.

## Link activation follow-up

Rendered Inline text now partitions its accessible children into ordinary text
and link nodes in reading order, without repeating the whole paragraph beside
the links. Adjacent styled runs of the same link coalesce into one action.
Each link carries its rendered label, URL and scaled native bounds; AX activation
uses the existing native link-click handler. GPUIO captures the installed source
generation/revision and validates current presentation identity, visibility/modal
scope and rendered mode before queuing navigation. No OCaml callback runs in paint
or in the accessibility delegate, and activation does not open URLs implicitly.

The gallery fixture includes bold text inside a link and a separate Unicode link.
The local external AX test observes exactly two links, activates their distinct
OCaml notices, collapses the document and verifies a retained AX reference cannot
activate it, then expands and continues through streaming/reset/remount. A focused
GPUI Base unit test verifies text partitioning, Unicode boundaries, adjacent styled
runs and invalid-range rejection. These tests cover ordinary Inline rendering.

Links split across rich inline objects, code-font fragments or multiple InlineFlow
elements still need complete accessible reading/focus coverage; the bold-link
fixture does not prove that case. Also keep heading levels, tables,
selected-text/range/copy, source pagination and VoiceOver acceptance open.

## Keyboard navigation and focus

Markdown links now carry source identities shared by their styled pieces. The
prepared document produces an ordered logical catalog independently of mounted
rows. Tab/Shift-Tab navigate it, Enter activates through the guarded callback,
Escape clears link focus, and pointer interaction clears keyboard link selection.
Append preserves an unchanged selected target; reset or a changed target clears
it. Native text selection is not used to represent keyboard link focus.

Inline's transparent semantic children now use actual GPUI Label/Link elements
instead of synthetic leaves. This preserves reading order and allows the selected
link to claim accessible descendant focus while the document retains real keyboard
focus. The renderer outlines the selected link. Elements are created only when
accessibility is active. Offscreen navigation first reveals the containing virtual
block and then makes a one-shot GPUI autoscroll request for the link's text position.

Actual macOS key delivery traverses eight links through six streamed findings,
asserts AX focus and the distinct queued OCaml destination at each link, checks
the distant final link's bounds lie inside the viewport, reverses direction and
uses Escape to restore document focus. The focus-outline screenshot was inspected.
The existing Unicode/read-only/collapse/reset/remount checks continue to pass.

GPUI layout tests traverse 60 links forward/backward inside one tall virtual block,
proving link-position reveal instead of merely aligning that block's bottom.
Catalog tests cover formatted/code spans, references, list/table placement,
adjacent same-URL links, nonwrapping boundaries, append retention and replacement.
The broader text suite passed 146 tests before the additional tall-block test;
the tall-block test also passes independently.

That suite exposed an existing idle-test failure: the previous commit also rendered
the README seven times during initial settlement. The strict parser-settlement test
now uses deterministic asynchronous-sized text without image-loading work. A separate
README test retains the image fixture and verifies no additional renders across
three subsequent simulated seconds. Both pass; this does not replace the release's
real-process idle/resource measurements. Direct AX focus requests and full image/
inline-object link behavior remain open, as does VoiceOver acceptance.
