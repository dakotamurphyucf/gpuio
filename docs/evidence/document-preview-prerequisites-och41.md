# Document preview prerequisites — OCH-41

This evidence covers the pinned GPUI Base `TextView.max_lines` adaptations.
The subsequent [public API slice](document-preview-api-och41.md) adds OCaml
configuration, queued observations and a gallery preview. The prerequisite tests
below are separate from that end-to-end integration evidence.

The upstream limit is a height budget measured in body-text line heights. It
applies to fit-content documents, not the widget's scrollable mode. Headings,
paragraph spacing and embedded content consume that budget; it is not a count
of Markdown source lines. Existing whole-line clipping is retained, including
the upstream treatment of a single oversized line.

## Reproduced defects and adaptation

The TestPlatform fixture mounts the production Base widget with prepared
Markdown, links and a real native code-block button. Tests first reproduced:

- Tab/Enter reaching a visually clipped link.
- `is_clamped` remaining true after removing the limit.
- Clipped links remaining exposed in the accessibility tree.
- A clipped code button accepting an explicit accessibility action.
- A previously focused code button accepting Enter after re-clamping.
- Keyboard focus getting stuck on that clipped control after its tab stop was
  removed.

The native adaptations now maintain frame-local painted link eligibility for
previews, while retaining the full logical link list for ordinary scrolling
documents. Removing the limit, enabling scrolling, or installing short content
updates overflow. A newly hidden active link loses activation eligibility.

`TextView` uses the final whole-line boundary for drawing, pointer hit-testing
and accessibility bounds. GPUI's new scoped prepaint helper refines only the
hitboxes created by that subtree. Its accessibility subtree helper preserves
node identities and hidden descendants; frame finalization removes hidden
subtrees' explicit action handlers and fallback click/focus targets. Expansion
rebuilds those mappings normally. The subtree walk stops before earlier sibling
subtrees rather than scanning every earlier document for each preview.

An opt-in paint scope suppresses fully clipped interactive Divs' keyboard
listeners and tab stops. It still paints descendants to cancel their own pending
clicks, and reports a clipped focus owner so `TextView` can return focus to its
visible document container. Ordinary scrolling does not opt into this policy.
Custom extension elements that register input directly must still enforce their
own input contract; this is not a general promise about arbitrary plugin code.

## Validation scope

The fixture is `rust/native/src/document_clamp_test.rs`, enabled through
`native-image-tests`. It covers forward/reverse link traversal, re-clamping an
active link, source replacement, scrollable-mode restoration, accessibility
visibility/actions and stable identity, independent neighboring semantics,
custom-control activation/restoration, keyboard focus recovery, pending key
press cancellation, and pointer behavior at a whole-line boundary.

Commands use the repository's isolated environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
```

The final full native suite passes **824 tests**, with two existing private-D-Bus
tests ignored on macOS. All **eight** focused preview tests pass. Strict Clippy
for native/protocol, all targets with image/canvas test features and `-D warnings`,
passes. `dune build -j 2 @runtest @fmt examples/gallery/main.exe` also passes.
A fresh installed public-gallery consumer passes with `run=False`:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery \
  --workspace /private/tmp/gpuio-document-clamp-gallery-20261003
```

The official `./scripts/gpuio check-fmt`, catalog source audit and `git diff --check` pass. Detailed baseline/fix logs are local under
`scratch/agents/root-20261003-release-notices/document-clamp-*.log`.

Both fork patches reconstruct exactly from hash-verified pinned archives:
**233 GPUI Base files** and **156 GPUI files** match byte-for-byte, excluding only
the local root `Cargo.lock` artifact in each tree. Upstream pins are unchanged.
Patch SHA-256 values in `third_party/sources.json`:

- GPUI Base: `f888929bbf947a6544082e1d0baf9a0c25dea5322006c47c6d49a2920c5f868c`.
- GPUI: `ceb13a69b2b49e48ba76ccd94992b4881815c047d87cf1dfa33ad4774428984a`.

Reconstruction commands (scratch archive/output locations are evidence, not build
inputs):

```sh
python3 scripts/vendor_gpui_base.py \
  --archive scratch/agents/root-20260912-milestones/gpui-kit-84f57fd.tar.gz \
  --output scratch/agents/root-20261003-release-notices/clamp-base-complete
python3 scripts/vendor_gpui.py \
  --archive scratch/agents/root-20260928-m7/zed-a57ba9b.tar.gz \
  --output scratch/agents/root-20261003-release-notices/clamp-gpui-complete
```

These are macOS arm64 TestPlatform checks. They open no AppKit window and do not
establish physical keyboard, clipboard, VoiceOver, IME, GPU or Linux desktop
acceptance. Public API/bridge/gallery evidence is recorded in the subsequent slice; OCH-17
qualification remains open.
