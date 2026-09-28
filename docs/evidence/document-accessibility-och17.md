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

Remaining release requirements include accessible and keyboard Markdown link
activation, macOS heading-level exposure, table reading semantics, native selected
text/range APIs, copy, modal/stale-action cases, bounded source pagination through
assistive tools, and actual VoiceOver reading/navigation. Current links expose
their visible text only. No hidden duplicate raw source substitutes for these
requirements. Linux desktop accessibility remains deferred to OCH-47.
