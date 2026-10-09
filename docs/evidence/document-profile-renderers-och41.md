# Installed document profiles — OCH-41

Local macOS arm64 evidence on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`
plus the working tree, following the [worker checkpoint](document-profile-workers-och41.md).
This is production-host behavior on GPUI TestPlatform. No OS windows, physical
keyboard/IME, VoiceOver or GPU acceptance is claimed.

## Implemented

Native document requests bind registered profile metadata and include observer
identity in worker equality. Prepared text, extensions, native renderers, complete
highlight styles and resource accounting install together. Code/table renderer
slots can replace declarative actions, while a declined slot retains their fallback.
Markdown block and inline plugins render through the installed SDK adapters.

Installed event contexts carry immutable source/configuration/handler provenance.
Property/handler/reset/clear/unmount/eviction transitions revoke old sinks; an old
picture displayed during an append retains its old revision. Paint gates visibility
and inherited input policy. Pointer guards remain separate from keyboard/AX guards.
Render/worker failures are queued once; guarded input panics close immediately and
report at the next host boundary. Native alert fallbacks preserve failure visibility.

Image closure refreshes retain the installed profile/parser identity and selection.
Sinks, plugin contexts, renderer closures and cached highlighters retain the shared
resource charge independently of event revocation. Weak transport/owner references
avoid keeping a window or transport alive. Last-holder release is tested; these
reservation units do not measure RSS or sandbox trusted plugin allocations.

## Validation

The seven new tests use a statically linked test-only SDK factory and the actual
Session/Host/worker/Presentation path, without replacing the production registry:

- Markdown/HTML code/table renderers, native mouse/AX events and complete custom
  font weight/strikethrough installation.
- Property replacement, handler-only rebinding, clear/reinstall and unmount while
  old Presentation/callback objects remain retained.
- Source reset before painting, native inline controls and once-only plugin errors.
- Pointer-disabled keyboard/AX operation, inert input rejection/restoration and
  once-only input-panic reporting.
- Real registered image refresh preserving selection/profile identity, followed by
  teardown proving charges survive until the last cached sink/highlighter drops.
- Native block-plugin controls and old/new source revision stamps during append.
- Configure, Highlight and Render failure stages with silent repeated redraws.

Full native **869 passed**, with two existing private-bus tests skipped on macOS.
Document SDK **15 passed**; extension SDK **5 passed**, including a new retained
resource test proving closed sinks cannot deliver while their accounting stays alive.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-native --features native-image-tests,native-canvas-tests --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --offline --locked -j 2 \
  -p gpuio-document-sdk -p gpuio-extension-sdk
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --offline --locked -j 2 \
  -p gpuio-native -p gpuio-document-sdk -p gpuio-extension-sdk \
  --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests \
  --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio check-fmt
python3 scripts/audit_component_catalog.py
git diff --check
```

Strict lint initially reported one nested `if`; it was collapsed without changing
behavior. Final strict lint, gallery link, formatting, structural catalog audit and whitespace
checks all pass.
A first targeted test filter matched zero cases and was corrected to `profiles::`;
the reported seven cases above actually executed. Test fixture compile corrections
used `CodeBlock::from_code`, the image-source namespace and the required block
plugin inline fallback. No new dependency pins or vendor changes were made.

## Remaining acceptance

Add a documented typed profile package, public gallery example and independent
installed consumer, then application-default inheritance/reset semantics. Complete
virtual offscreen focus/clipping and broader plugin selection/search/copy/accessibility
qualification. The OCH-41 catalog and OCH-17 physical macOS, resource/performance,
notices/distribution/API/review/required Linux nongraphical gates remain open.
Neither the ticket nor milestone is complete.
