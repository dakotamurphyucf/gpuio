# Native menu-bar artwork — OCH-41 / OCH-17

Local macOS arm64, 2026-10-07, working source based on `98e4eedf`. This
checkpoint extends the existing public SVG menu-icon API to native menu bars.
It qualifies the cases below, not the entire catalog or release.

## Implementation

`View.with_menu_item_icons` accepts direct platform bars and platform context
menus. Item paths distinguish repeated commands and nested submenu rows. Rich
interactive rows, raster handles, separator targets and duplicate/unknown paths
remain rejected. No protocol tag or dependency version changed. Linux keeps the
drawn fallback; no new-source Linux execution or GUI acceptance is claimed here.

Rust observes the existing image workers and installs validated, bounded template
bitmaps through the GPUI macOS adapter. Snapshot comparison includes ready image
identity, position and transform, avoiding native menu replacement on unrelated
renders. The [contract](../design/native-popup-menu.md#menu-bar-artwork) states
source ownership, independent 8 MiB bar/popup payload budgets, AppKit copying and
active-window ownership. No additional freeze-during-tracking guarantee is added.
The [Feedback walkthrough](../../examples/gallery/feedback_page.md) explains the
OCaml implementation and Bonsai command path.

Actual opening revealed an existing disabled-state defect: AppKit's automatic
validation overrode `setEnabled(false)` because GPUI had a generic command
listener. The native regression fails after `NSMenu.update()` on the original
adapter. Disabled rows now have no registered action and use the GPUI delegate
selector even for OS edit actions. Validation therefore rejects them; enabled
routes retain their original behavior. See the [adapter design](../design/gpui-macos-adaptation.md).

## Validation and boundaries

- OCaml `dune runtest -j2 test/view_api` passes, including root/nested/repeated
  icon slots, empty clearing and rejection. The gallery builds successfully.
- Portable integration tests pass: menu admission **5**, bitmap shape/owned
  clone retirement **2**, action-registry retirement **3**.
- Actual AppKit `native_menus` passes nested/disabled bitmap content, 16-logical-pixel
  templates, registration release before readiness, transform/source updates,
  retained old snapshots, unchanged-render object identity, command delivery,
  clearing and surviving-window restoration. Existing menu families also pass.
  The clipboard is captured, restored and verified around the fixture.
- The public gallery `--section native-bar` passes actual pointer opening and
  activation, top-level and nested SVG silhouette checks, clear/restore,
  disabled command validation, independent active-window menus, page removal and
  remount. Ready checkmark matches are at least 0.966; cleared rows are below
  0.74. This measures the expected silhouette, not merely non-background pixels.
  Both windows close and the child exits zero. No clipboard or VoiceOver actions
  are performed by this gallery section.
- The native library suite passes **1,187 tests, two existing skips**, including
  the popup bitmap/transform/budget regressions after their shared conversion
  refactor. Strict workspace/all-targets Clippy, workspace/vendor formatting,
  example documentation inventory and catalog audit pass. Existing upstream
  deprecation warnings remain.
- Reconstructed pinned upstream sources match all **159 GPUI** and **19 macOS**
  files exactly, excluding generated Cargo.lock. GPUI patch SHA-256:
  `2711541338a3338ed70e2b380f2094677fc6f36c44f36c0704bdd3518c8eb17b`;
  macOS: `d86565e16ab8ae3d49877f53c621e263df32eead70c7a67cd9090a94472d92eb`.

Native fixture executable SHA-256:
`89e03a28a3d31c69238091ff5b13a5dd31226d00a2d4ae2568455417ca08e149`.
Final gallery executable SHA-256:
`b4564a543030a54c3c435b1cedc46b94d95085e43d158bbfd568bca7ae3b15bc`.
After that native fixture run, Clippy requested the equivalent test-only
`is_multiple_of(16)` spelling; production behavior is unchanged and the final
strict check compiles that test source.

This is not an independently installed-consumer run, a Linux check, a VoiceOver
reading/navigation result, an OS-theme/display-density matrix, a memory/RSS
qualification or a full-gallery pass. The new section restores its initial
preview stage to compose with other sections; only its standalone execution is
claimed here. Final-source hosted checks and consolidated acceptance remain.

## Reproduction

Use the repository environment with `GPUIO_JOBS=2`. Run each compiler pipeline
sequentially and do not compile during GUI/performance measurements.

```sh
./scripts/gpuio exec dune runtest -j2 test/view_api
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --test menu_action_registry --test menu_content --test menu_icons
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --features native-tests --test native_menus --no-run
./scripts/gpuio exec cargo test --offline --locked -j2 -p gpuio-native \
  --lib --features native-canvas-tests,native-image-tests
./scripts/gpuio exec cargo clippy --offline --locked -j2 --workspace --all-targets \
  --features gpuio-native/native-tests,gpuio-native/native-canvas-tests,gpuio-native/native-image-tests,gpuio-native/presentation-diagnostics \
  -- -D warnings
./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section native-bar --images scratch/native-bar
```

Execute the built `native_menus` binary under
`scripts/mac_clipboard.py`'s `preserved_clipboard()` context; the archived runner
records that invocation. The fixture exercises Copy. Foreground windows are
required for the real gallery input checks.

## Retained failures

An early fixture used sparse tree slots and correctly failed admission; it now
owns an isolated window/tree with consecutive slots. The first gallery probe
also exposed AppKit's first-menu naming convention, so the example now supplies
an application menu before Workspace. Early automation attempts queried a
closed menu, toggled a hover-open menu shut, or captured a submenu before its
opening animation completed. The final driver checks native ownership and
visible menu state without weakening its silhouette or disabled assertions.
Two state-wait errors in the driver were corrected against the actual public
model labels and asynchronous page mounting. The disabled validation failure
was reproduced separately in the native fixture and repaired in production.
Clippy's initial assertion-style failure is retained too.

[Reports and source](native-menu-bar-icons-och41/reports.tar.gz) have a
[SHA-256 manifest](native-menu-bar-icons-och41/manifest.json). OCH-41 and OCH-17
remain In Progress.

## Installed-consumer and driver-composition follow-up

The [header/consumer follow-up](gallery-header-layout-och41.md#independently-installed-consumer)
now qualifies this same menu-bar section against freshly staged public libraries
and an independent gallery build. Native opening, nested/cleared/restored
silhouettes, pointer dispatch, disabled state, active-window ownership and
remount pass; its child exits zero. This adds installed-consumer evidence beyond
the original local checkpoint above, without adding Linux, VoiceOver,
performance or whole-family acceptance.

The driver now discovers the new window instead of assuming its serial is two.
Two consecutive complete local exercises in one process pass, preserving the
initial preview stage between runs. This corrects test composition with earlier
window creation; it does not change application window names. The follow-up
archive retains the repeated-run log and the independently built executable's
identity.

## Full-suite fixture generations — 2026-10-08

Hosted run [37731811839](https://github.com/dakotamurphyucf/gpuio/actions/runs/37731811839)
at `de452dea` exposed a fixture-composition failure in `native_controls`:
the preceding command-isolation check had retired protocol window `(1, 1)`,
but the artwork check tried to open that same generation. The production Session
correctly returned `StaleHandle`. The standalone menu test did not establish the
same prior window history, so its earlier pass did not cover this composition.

The fixture now uses generation 2 for artwork. The standalone menu suite reserves
and closes generation 1 to match the full suite. The first corrected full run
passed artwork, then exposed a second outdated assumption: the later menu-context
fixture searched only generations 1 and 2. It now explicitly opens generation 3,
after artwork retirement. All fixtures still share the same Session and verify
surviving-window ownership; production generation validation is unchanged.

On the local physical macOS arm64 host, both final executables exit zero:

- Complete `native_controls`: 16.484 seconds, including the preceding command
  fixture, artwork, menu contexts and all subsequent control checks.
- Standalone `native_menus`: 4.265 seconds, including artwork, stale native actions,
  focused-window routing and surviving-window restoration.
- Strict native-tests all-target Clippy and workspace Rust formatting pass.

These are actual native-window/AppKit checks with programmatic test input and AX
queries, not VoiceOver speech/navigation or new physical-typing evidence.
Observation source is `9e6fea4c` with the accompanying three-file fixture patch;
the archive records both executable hashes and exact build/run commands. All
owned children were reaped. The rejected first build command (`native_menu`
instead of `native_menus`) and the intermediate failing control run are retained.

The [fixture archive](native-menu-bar-icons-och41/fixture-reports.tar.gz) includes
the original hosted failure excerpt, intermediate and passing local results,
source patches, build/lint logs and the hosted receiver reports. Its
[manifest](native-menu-bar-icons-och41/fixture-manifest.json) was verified against
every archive member. Updated-source hosted acceptance remains pending.
