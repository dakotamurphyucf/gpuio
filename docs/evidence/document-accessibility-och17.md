# Document accessibility — OCH-17

Partial release finding repair, 2026-09-28, macOS 14.5 (23F79), arm64.
The [contract](../design/document-accessibility.md) describes the full intended
behavior. This checkpoint does not complete document accessibility or OCH-17.

## Direct link focus — 2026-09-29

The external macOS gallery test reproduced a missing action: setting AXFocused
on the rich Markdown link did not focus it. Both ordinary Inline and rich
InlineFlow semantic links now advertise and handle Focus. They validate the
prepared source identity and URL, select the logical link and reveal it through
the existing document focus owner. No navigation occurs until a separate
activation. The host's native predicate rejects collapsed/source presentations,
stale installed revisions or source generations, retired owners and input scopes
blocked by the existing visibility/modal controller. No OCaml API, protocol or
AccessKit modification is needed.

The full `scripts/test_gallery.py --section documents` walkthrough passes on
macOS 14.5 arm64: rich/code-font and Unicode image-placeholder links accept direct
AX focus without navigation; Shift-Tab/Escape preserve the logical order and
document owner; ordinary streamed links accept focus and Enter navigation.
Retained collapsed links cannot steal focus or activate. Existing heading/table,
streaming/reset, code/diff, copy and repeated remount checks also pass. The focused
link test additionally checks the retained-node focus result after asynchronous
action delivery and distinguishes focus from the subsequent Enter destination.

The Base text suite passes **225 tests**, including the new identity/URL/guard
rejection, successful focus, reverse navigation and source-replacement regression.
The real native document suite passes, including installed-revision, collapsed,
source-mode and native-hidden predicate checks, and source-generation rejection
before/after presenter refresh during held reset installation. Strict native/
protocol Clippy (`--all-targets --features native-image-tests -- -D warnings`),
formatting and exact reconstruction of all 233 Base files pass. The cumulative
Base patch SHA-256 is
`621805197c758798969a388b20b04774cf6307da9592dda89477cc43e586ae7e`.

This checkpoint covers external AX requests and native guard behavior. It does
not establish an actual VoiceOver session or complete modal assistive-navigation
acceptance. Decoded/unlinked images, custom control reading order, selected-text/
range APIs and remaining release requirements stay open. The older direct-focus
gap descriptions below are historical and are superseded by this checkpoint.

## Initial stateless-text rendering — 2026-09-29

The earlier Base initial-render failure is repaired without relaxing its existing
`<= 2` assertion. A new keyed stateless TextView state was created with selection
disabled, while its view defaulted to enabled. Applying that initial value through
`set_selectable` during first layout emitted a redundant notification before the
asynchronous parse completed. The constructor now initializes the state with the
view's selection setting. Subsequent configuration changes still use the existing
setter and its selection-clearing/notification behavior.

The unchanged focused settling test passes. The broader Base `--lib text` run
passes **224 tests**, including rich-text layout, shared selection, virtualization,
copy, link navigation and idle-after-settling checks. The previous failure and
unchanged-baseline reproduction remain below as history. This is a virtual GPUI
render-count regression, not macOS application CPU/FPS or full idle-budget
acceptance. The new eight-line adaptation and cumulative Base patch reconstruct
all 233 files exactly, excluding the ignored standalone Cargo.lock.

The standalone Base test command uses the repository's existing local GPUI/Taffy
patches and test profile:

```sh
./scripts/gpuio exec cargo test --manifest-path vendor/gpui-base/Cargo.toml --lib text --offline -j2 \
  --config 'patch."https://github.com/zed-industries/zed.git".gpui.path="/Users/dakotamurphy/gpuio/vendor/gpui"' \
  --config 'patch.crates-io.accesskit_macos.path="/Users/dakotamurphy/gpuio/vendor/accesskit-macos"' \
  --config 'patch.crates-io.taffy.path="/Users/dakotamurphy/gpuio/vendor/taffy"' \
  --config 'profile.dev.debug=0' --config 'profile.dev.package."*".opt-level=1'
```

Use absolute paths for the current checkout when reproducing this local command.

## Reset preparation and newer collapse input — 2026-09-29

The post-reset collapse failure recorded below now has a deterministic native
reproduction and repair. A prepared generation installed after the user's Collapse
and unconditionally restored `initially_collapsed`, undoing the action. The native
presentation now records which source generation supplied the current collapse
state. The shared pointer/keyboard/accessibility operation stamps the current
lease generation, including before the presenter has refreshed. Installation
restores the configured default only when no interaction has already established
state for that generation. Later resets still restore defaults; append preserves
the current choice. No worker scheduling or public wire/API contract changes.

`document_reset_test.rs` holds actual prepared-result installation through a
`native-tests`-only gate and sends Enter through native GPUI focus/input routing.
The first reproduction failed when collapsed `true` became `false` on installation.
The repaired test passes twelve code/Markdown/diff × initial-state × input-order
cases, with input both before and after presenter refresh. Each also verifies
append retention and the next unacted reset's default. Publication and input in
the before-refresh case share one native update; neither test depends on sleeps
or worker speed. This is GPUI key injection, distinct from OS keyboard acceptance.

The complete `native_document` regression passes. The unchanged full gallery
`--section documents` flow also passes locally, including the previously failing
Reset diff → Collapse/Expand → Markdown sequence, native scrolling to code,
rich-link checks and repeated remounts. Its earlier fence-reveal timeouts are
retained below; this run does not independently establish their cause. Failure
diagnostics now dump the accessible tree and, when `--images` is supplied, capture
the application window before teardown.

Validation on macOS 14.5 arm64: native document build/run, gallery build plus
`@fmt @test/gallery/runtest`, full document gallery with screenshots, strict
feature-enabled Clippy, default-feature Cargo check, Rust formatting and Python
compilation. Relevant commands are the rich-link commands below with gallery
`--section documents --images <directory>`. No Linux GUI, full installed-consumer
rerun or release/performance acceptance is inferred from these checks.

## Rich links across rendered fragments — 2026-09-29

The public document fixture now includes one link spanning ordinary, bold and
inline-code text, plus a linked safe image placeholder with a Unicode alternative.
Previously the first appeared as two partial AXLinks and the second appeared only
as placeholder text. The frame-local rich-paragraph collector now exposes one
logical action per link in source reading order, using its prepared full name and
actual fragment geometry. Source identity distinguishes links with the same URL;
the URL also participates in accessible identity when a target changes. Wrapped
fragments contribute unioned accessible bounds, individual focus outlines and a
first-fragment keyboard reveal target. The existing queued navigation handler
still validates the installed native presentation.

Projected plain-text fallbacks do not duplicate the link's alternative name.
Native custom descendants retain their own semantics and input. Their premeasured
element paths remain unchanged; accessibility-only visual containers forward the
existing text/image layout rather than adding layout boxes. No text is reshaped
for this collector, and no asset fetch or document-history cache is introduced.

The focused gallery, native document/highlighting regressions, build/format and
strict lint pass locally with the commands below. The gallery checks exact logical
link counts/names, intervening text order, AXPress destinations, eight Tab/Enter
targets, distant-link reveal/geometry, Shift-Tab/Escape and retained actions
after collapse. The native document suite also covers
selection, clipboard, registered-image rendering, streaming and disposal; that
rendering check does **not** establish decoded-image accessibility acceptance.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt @test/gallery/runtest -j2
python3 scripts/test_gallery.py --section document-links
./scripts/gpuio exec cargo test -p gpuio-native --test native_document --test native_highlight_document --features native-image-tests --locked -j2 --no-run
# Run each emitted native executable separately under a bounded process-group watchdog.
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol --all-targets --features native-image-tests --locked -j2 -- -D warnings
```

Three focused Base unit tests pass for source identity, Unicode names, native
reading-order barriers, wrapped union bounds and first-fragment geometry. The
full Base text run had **148 passes and one failure**: the existing stateless
Markdown initial-render assertion observed three renders against a limit of two.
An isolated run and a fresh archive of unchanged `dc25013` reproduce the same
failure. The threshold was left unchanged. The initialization repair above now resolves
this failure; this historical run was not a passing full Base suite. A baseline build shared the
same Cargo artifact filename, so the final current-source focused run explicitly
recompiled and verified that all three new tests executed.

The longer `--section documents` walkthrough passed during development but is
not consistently passing: later runs timed out locating a fence after returning
from Diff, or waiting for Expand after Reset diff followed by Collapse. Added
geometry diagnostics produced another pass; a two-direction/current-viewport
search did not establish a fix and was not retained. Inspection found generation
installation resets collapse state in `Presentation::accept_ready`; a delayed
reset could overwrite a newer collapse action. The follow-up above reproduces
and repairs that race; the earlier observations remain recorded here. The dedicated `document-links`
section isolates the complete new link checks, and the full walkthrough still
calls the same checks rather than skipping them.

The cumulative Base patch reconstructs all 233 vendored files byte-for-byte;
the only additional local file is its ignored standalone Cargo.lock. Remaining
acceptance includes decoded/unlinked images, arbitrary custom controls, direct
AXFocus, selected-text/ranges, stale/modal cases and actual VoiceOver navigation.
This checkpoint does not complete OCH-17, OCH-41 or Linux desktop qualification.

## Table header relationships — 2026-09-29

The macOS adapter now exposes current column/row header arrays and the column
headers' shared row/group container. These are references to existing painted
semantic nodes, correlated with cell column indices. Queries filter hidden nodes,
stop at nested tables and do not materialize offscreen history or retain another
copy of the table. [Apple's API](https://developer.apple.com/documentation/appkit/nsaccessibility-c.protocol/accessibilitycolumnheaderuielements)
and the [Core AAM draft mapping](https://www.w3.org/TR/2026/CRD-core-aam-1.2-20260923/#role-map-table)
describe these macOS relationships.

The external gallery's focused document walkthrough passes: two column headers
are CFEqual to the actual first-row cells, AXHeader is the first row, and there
are no semantic row headers. Existing count/index/Unicode checks repeat after
collapse and across three remounts. Keyboard links, source/diff controls,
streaming, native scrolling and shutdown continue to pass.

The managed-table regression also passes against its 100k logical-row source.
It receives exactly the two painted header cells and their shared container,
repeats the check at logical row 50,001, and verifies a retained hidden-table
reference no longer exposes the old headers. Existing OS keyboard/clipboard,
selection, marked-text/grapheme, style and lifetime checks pass in that run.
Its existing header container now has a RowGroup role; its element identity is
preserved. These fixtures have no semantic row headers, so that array is empty.

Local macOS 14.5 arm64 validation passes:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j2
python3 scripts/test_gallery.py --section documents
./scripts/gpuio exec cargo test -p gpuio-native --test native_table_host --features native-image-tests --locked -j2 --no-run
# Run the emitted native_table_host executable under a bounded process-group watchdog.
./scripts/gpuio exec cargo clippy -p gpuio-native -p gpuio-protocol -p gpuio-table-adapter --all-targets --features gpuio-native/native-image-tests --locked -j2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @fmt @test/gallery/runtest -j2
```

Both GUI processes exit zero and are reaped. The sixth ordered AccessKit patch,
`table-headers.patch`, reconstructs exactly from the pinned crate and previous
five patches. No dependency versions change. Logs use `table-headers` in the
implementing agent's ignored notes directory. This does not add AXColumns or
establish complete VoiceOver column navigation, selected-text/range APIs,
rich/image-link acceptance, or consolidated hosted/consumer release acceptance.

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
