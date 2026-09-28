# Document accessibility — OCH-17

Partial release finding repair, 2026-09-28, macOS 14.5 (23F79), arm64.
The [contract](../design/document-accessibility.md) describes the full intended
behavior. This checkpoint does not complete document accessibility or OCH-17.

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

Remaining release requirements include keyboard Markdown link navigation,
rich-fragment link identity, macOS heading-level exposure, table reading semantics, native selected
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

Keyboard traversal/visible focus remains unimplemented. Links split across rich
inline objects, code-font fragments or multiple InlineFlow elements need a shared
logical identity review; the bold-link fixture does not prove that case. Also keep
heading levels, tables, selected-text/range/copy, source pagination and VoiceOver
acceptance open. Synthetic AX Click support alone is not keyboard or screen-reader
navigation acceptance.
