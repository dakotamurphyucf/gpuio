# Presentation component evidence (OCH-33)

## Semantic, form and stateless presentation foundation

Local macOS arm64, 2026-09-25; stock project OCaml 5.3/Bonsai v0.17 pins and
isolated toolchain. This checkpoint is **partial OCH-33 implementation**. Avatar
asset fallback, native loading indicators, rating and complete theme/scale/content/
lifetime acceptance remain. No OCH-33 capability or hosted/Linux result is claimed.

`Accessibility` validates bounded UTF-8 labels/help/errors, heading levels and
live-region priorities. `View.with_accessibility` and its Bonsai specialization
preserve identity and reject ambiguous/unsupported native roots. Appended wire
operation 39 sets or clears metadata independently of styles and editor config;
previous tags are unchanged. Native admission is atomic and charges retained
metadata storage to the existing tree budget. Text is bounded to 4096 bytes per
string; the standalone decoder bounds configuration bytes to 16384 and rejects
invalid strings/combinations, truncated/trailing data and oversized declarations.

Three independent OCaml/Rust fixtures cover field data, full update/reset messages
and Status/Alert/Heading/Link roles. Tests prove metadata-only changes do not create,
remove or reset controls, unchanged semantics emit no transaction, and invalid
native updates roll back preceding text changes and revision/accounting. Removal
reclaims retained storage. AccessKit metadata preserves native role/actions,
read-only state, and adds required/invalid properties. Explicit synthetic label,
description and error relationships use the existing GPUI builder; no fork patch
or new callback runtime was needed.

The native `native_presentation` test passed actual AppKit field label/help/error/
required output, updates and reset during marked-text composition, exact editor
snapshot retention, AXLink and keyboard activation, and editor/tree disposal.
AccessKit's invalid/relationship data is not exposed as a corresponding macOS AX
attribute in every case; do not infer a full assistive-technology audit from the
programmatic label/help/required checks. The public example adds real keyboard
delivery through macOS's event API. No physical monitor-scale transition or Linux
GUI acceptance is claimed here.

`Form.field` adds application-owned label/help/error layout around supported native
controls. Its internal keys preserve the control when optional help/error or layout
changes. Validation and editor ownership remain with the caller; the static label
does not install an extra focus target or a click-to-focus action.

`Presentation` implements label, badge/tag/marker, Link, separator, group/settings,
description list, empty state, alert/banner, shortcut/status bar and attachment/
message/bubble/tool-result compositions. Concrete light/dark appearances do not
require new theme tokens; applications can provide custom colors or tokens.
Named slots preserve child identities. These helpers create only ordinary text/
container views and explicit caller actions; they own no independent controllers,
timers, assets or documents. Expect tests cover empty/localized/10,000-character
content admission in both themes, no-op updates, optional-slot body identity,
latest action dispatch/disabled fencing and explicit live/description semantics.
This is admission/identity coverage, not proof that every long-text layout is good.

The [public example](../../examples/presentation/README.md) composes a settings
form, details, chat message, tool result and attachment through ordinary public
Core/Bonsai/Eio APIs. Its self-test waits for acknowledged frames across dark/light,
error insertion/removal and verifies exact native editor snapshots and shutdown.
`scripts/test_presentation.py` separately exercises real keyboard input, native
field help/error, retained text after theme changes, AXLink, card actions and close.
All owned children are reaped on success or failure.

Initial screenshot inspection caught compressed card actions and incorrect light
control defaults. Stable action wrappers now retain intrinsic width; the example
and link set their intended colors. The external AX test guards action dimensions.
Corrected dark/light screenshots were inspected at 1040×860 logical window size
on the development display. They are checkpoint images, not the final OCH-46 demo.

![Dark component studio](../images/presentation-dark.png)

![Light component studio](../images/presentation-light.png)

Passing local commands, with `GPUIO_JOBS=2`:

```sh
./scripts/gpuio exec dune build -j 2 @all @runtest @fmt
./scripts/gpuio exec cargo test --workspace --locked -j 2
./scripts/gpuio exec cargo clippy --locked --workspace --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -j 2 -- -D warnings
./scripts/gpuio exec dune exec -j 2 examples/presentation/main.exe -- --self-test
python3 scripts/test_presentation.py --images scratch/presentation-images
./scripts/gpuio exec cargo test --locked -p gpuio-native --features native-tests --test native_presentation --test native_controls --test native_editor --no-run -j 2
```

The three built native executables were then run directly with a 60-second timeout
per target and all passed. This also revalidates existing controls, menus/palette,
progress/toasts/pointer behavior and the editor's grapheme, clipboard, focus,
revision, resize and AppKit text-client paths through the shared semantic wrapper.
Native marker output includes `GPUIO_PRESENTATION_SEMANTICS_OK`,
`GPUIO_PRESENTATION_AX_OK`, `GPUIO_CONTROLS_NATIVE_OK`,
`GPUIO_CONTROLS_MACOS_AX_OK`, `GPUIO_EDITOR_NATIVE_OK` and
`GPUIO_EDITOR_MACOS_TEXT_CLIENT_OK`.

CI definitions include native/public/AX checks for this foundation, but the milestone's
consolidated hosted run and merge are still pending. Continue with avatar, loading
and rating; extend the example and complete OCH-33's acceptance before marking it
Done. OCH-46 remains the final integrated chat showcase after all component tickets.

## Native loading indicators

The next checkpoint adds `Loading.Kind`/`Config` and `View.loading` (also in
`Gpuio_bonsai.View`) with native Skeleton, Shimmer and Spinner presentations.
Wire kind 34 and operation 40 are appended; no existing tags or capability mask
change. Labels are nonblank UTF-8 without NUL, at most 4096 bytes. The native period
is 100..60000ms, defaults to 1200ms and rounds up at the public API boundary.
Explicit `animated=false` and application reduced motion produce static output.

Native paint is bounded to one skeleton quad, three shimmer quads or twelve spinner
strokes. Size/color come from ordinary styles. The existing radius-capture wrapper
transfers computed placeholder corners to paint, including the clipped shimmer
highlight. The leaf owns no editor/controller, worker task, event callback or
OCaml clock. GPUI advances its animation while visible; configuration changes
retain the native node and do not invent a completed fraction.

The independent `loading-request.hex` fixture contains all three kinds and minimum,
default and maximum periods. OCaml/Rust checks agree on its exact bytes. Rust
rejects every truncated prefix, trailing bytes, invalid UTF-8, oversized/blank/NUL
labels and invalid periods. Expect tests cover validation/rounding, no-op updates,
no callback binding and same-leaf shape/static changes. Native admission rejects
missing configuration, handlers and child text atomically, preserving previous
state/accounting; disposal reclaims storage.

Actual macOS native checks pass for all three cycles advancing without tree commits,
size/color/radius overrides, indeterminate AX roles, reduced/static whole-window
idle, parent-hidden idle, resume and weak-state/tree disposal. A real minimize/
restore check also passes: the minimized window stops rendering and animation
resumes after restoration. Markers are `GPUIO_LOADING_NATIVE_OK` and
`GPUIO_LOADING_MINIMIZED_OK`.

The first external AX check found that hidden indicators stopped painting but
remained exposed through accessibility. The shared semantic wrapper now writes
AccessKit's hidden state from the existing native visibility policy. A hidden
structural element without a role gets a Group node so that its descendants can
be hidden too. This is an internal semantic fix, not a new protocol field. Unit
coverage checks that representation, and both native and external tests now prove
the indicators disappear and return in the actual macOS AX tree. External tests
also assert that indeterminate loading has no numeric AX value.

Component Studio now exposes all three loading forms plus Hide/Show and Static/
Animate controls. Its public self-test retains the editor through these updates.
Both themes were captured and inspected; the screenshots above show the expanded
example. Rounded shapes, centered static shimmer and radial spinner are visible.
This does not claim a physical monitor-scale test or a full VoiceOver audit.

Passing checks use the isolated jobs=2 toolchain: full Dune `@all @runtest @fmt`,
Rust workspace tests, default and native-image/native-canvas all-target Clippy,
public `--self-test` and `scripts/test_presentation.py`. Native presentation is
rebuilt with the new loading checks. Native presentation, controls, editor and
container-query executables were built with `native-image-tests` and run directly
with a 90-second timeout each; all pass. The query regression includes retained
editor/IME isolation, native hidden accessibility, scaled layout and its existing
256-query workload. All owned native and public test windows/processes were
closed/reaped; no hosted or Linux GUI result is claimed.

Avatar fallback, rating and the complete OCH-33 family acceptance remain pending.
The existing presentation CI step automatically includes the extended native/public
tests; consolidated hosted gates and milestone-05 merge still follow local scope.
