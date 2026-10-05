# Selection controls: pinned behavior review

OCH-41 review of Longbridge GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The nine checksum-pinned
`base-{checkbox,radio,radio-group,switch,toggle,toggle-group}` and
`component-{checkbox,radio,switch}` snapshots in `sources/` are the source
inputs. This review is not native acceptance of the entire family.

| Source behavior | GPUIO mapping and difference |
| --- | --- |
| Checkbox checked/mixed/disabled; native click and accessibility action | `View.checkbox`, `Check_state`, `on_toggle`; an intent is applied to the current OCaml model rather than returning a Boolean computed from stale rendered state. Mixed activates to checked. |
| Switch controlled Boolean, disabled, label and accessible name | `View.switch`, `checked`, `on_toggle`, `accessible_name`; the same current-model intent contract. |
| Radio and radio-group controlled selection | `Choice.Config` and `View.radio_group` use stable IDs instead of styled-layer indices. Disabled options may remain selected but cannot be activated. One native Tab stop and Arrow/Home/End navigation are an explicit GPUIO behavior. |
| Toggle button with persistent pressed state | `Command.create ~checked` plus `View.command_button` supplies native button/toggled semantics. `Style.State.Checked` is persistent selection; `Pressed` is the transient pointer/key press. The callback requests an application action. |
| Toggle group | The base group is a container with `Toolbar` role and orientation; it has no selection model, automatic exclusivity or arrow navigation. The styled component-button ToggleGroup adds a Boolean-vector callback and segmented styling; see the button review. A command registry plus a row/column supplies composition. Toolbar role/orientation is now implemented locally; desktop validation remains pending. |
| Base child composition and styled text labels | `checkbox_with_label`, `switch_with_label` and stable-ID `radio_group_with_labels` add checked passive compositions; string constructors remain available. Arbitrary interactive children are deliberately excluded; native acceptance is pending. Optional `Control_appearance` adds bounded indicator/mark styles, with native acceptance still pending. |
| Styled tooltip, size, color and label placement | Explicit tooltip composition and `Control_appearance` size/gap/label order/part colors cover the public shapes; actual native evidence remains required. |
| Styled checkbox role override / independent radio / custom Tab order | Typed `View.radio` / `radio_with_label` supply distinct Radio semantics and application-owned selection. `Tab_order` covers checkbox/switch/radio/managed-group order and stop policy. Semantic `Radio_group orientation` supports arbitrary composition. Local checks and public example exist; real macOS acceptance remains open. |

The [control appearance contract](../design/control-appearance.md) now connects
checked Core values to optional View arguments, paired transactions and scalable
native paint. Indicators have explicit size, switch width, gap, label position
and bounded selected/mixed/disabled part colors. Omitted appearance preserves
18×18 checkbox/radio and 30×18 switch defaults. The Controls gallery demonstrates
custom/default presentation and interaction policies. Native scene and
transaction checks pass; further integration and actual GPU/AX acceptance remain
open. Rich labels are implemented locally under the [label contract](../design/control-labels.md),
with native acceptance pending. The [checkable navigation contract](../design/checkable-navigation.md)
now connects standalone radio composition and Tab order across the public API and
native bridge. The Controls gallery demonstrates both; OS validation remains open.

## Toolbar contract for composition

The public API supplies `Accessibility.Orientation.t = Horizontal | Vertical` and
`Accessibility.Role.Toolbar of Orientation.t`. The role is valid only on an
ordinary container; wrap the row/column inside a command scope. Orientation is
semantic metadata and does not change layout, invent roving focus, or own
selection. Native buttons retain their existing focus, actions, checked state
and asynchronous command dispatch. Applications choose single, optional single
or multiple selection through their model. Disabled-state reducers should reject
already queued requests using the current policy.

Role tag 14 is appended to the existing bounded semantic metadata codec; orientation
tags are 0/1. Existing encodings remain unchanged. This extends the paired
OCaml/native experimental release: an older decoder rejects the unknown role;
this does not promise older binary interoperability. There is no new operation
or callback transport. Use independent byte fixtures, placement rejection checks
and native AccessKit metadata checks. Real macOS toolbar/button navigation and
accessibility acceptance remain required separately.

The public Controls gallery now demonstrates exclusive alignment, independent
formatting toggles, horizontal/vertical toolbars, disabled-request handling and a
mixed master checkbox. Its reducer rejects queued requests after disabling,
applies rapid toggle intents to current state, and preserves exactly one
alignment even when activating the selected item. Model tests establish request
semantics; they do not establish GPU, focus or AX behavior.

## Local validation — 2026-10-01

On the macOS development checkout:

- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe`
  passes after updating two exhaustive matches for the appended role. The new
  expect tests cover the independent byte fixture, incompatible placement and
  current-model selection behavior.
- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native -p gpuio-protocol --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests --lib --test accessibility`
  passes: 469 native library tests with two existing ignores, two native semantic
  transaction tests, 37 protocol library tests and eight semantic codec tests.
  Toolbar checks cover both orientations, unchanged child identity, invalid
  child-role atomic rollback, metadata reset and zero session retention on removal.
  AccessKit node assertions do not establish actual macOS AX behavior.
- `scripts/test_gallery.py --section selection` is authored and included in
  `all`; Python compilation and CLI discovery pass. Its real desktop assertions
  are **unrun**: toolbar AX roles/orientation/identity, toggle/mixed values,
  Space/Return/Tab input, disabled activation and page retirement.
- Strict native/protocol all-target Clippy with the same test features and
  `cargo fmt --all -- --check` pass. The catalog checksum/inventory audit passes.
- A fresh installed-package gallery build passes with `run=False`. After the
  example's command IDs were separated from displayed labels, the final model
  and preview files were recopied into that independent consumer and rebuilt
  successfully against the same installed packages. The full repository
  OCaml test/format/gallery command also passes after that refinement. This is
  local build evidence, not a clean-machine or native runtime qualification.

The pinned AccessKit macOS adapter maps a button with a toggled value to
`AXCheckBox` with toggle subrole, and toolbar to `AXToolbar`. The driver follows
that mapping; these are source-derived expectations, not an observed desktop
pass. Full Linux desktop qualification stays deferred under OCH-47.

The appearance integration now also has a passing fresh installed-gallery build.
The physical `native_image_views` appearance fixture links and passes strict
Clippy; `--section control-appearance` is included in the public gallery driver.
Python compilation/CLI discovery and offline pixel-coordinate checks pass.
Both new desktop scenarios are **unrun**; see the
[appearance acceptance preparation](../design/control-appearance.md#native-acceptance-preparation--2026-10-01).
These do not close actual GPU/input/AX, rich-label, standalone-radio or Tab-order
acceptance gaps.

Rich-label local integration now passes four Core expect cases, four native
admission/fixture cases, the full Dune/Rust suites and strict Clippy. The expanded
native library passes 477 tests with two existing skips. Production-View checks
cover single activation/paint, label order, disabled avatar fallback, retained
radio identity across reorder, animated-label wakeups and cleanup. A fresh
installed-gallery build passes with `run=False`. The expanded public appearance
driver is authored and Python-compiles; its rich-label desktop assertions remain
unrun. See [rich labels](../design/control-labels.md) for commands and limits.

## Radio semantic and request contract

The pinned `base-radio` reports both toggled and selected state, plus position
and set size when supplied by its group. The native group now derives those
fields from the current ordered `Choice.Config`: position is one-based and
includes disabled options; size is the entire current collection. Disabled
selection is still a selected value, even though activation is forbidden.
Reorder changes positions without changing option IDs or native focus ownership.

The pinned standalone Radio suppresses activation while checked. Our managed
group deliberately queues selection **requests**, including a request matching
the currently committed native value. With committed A, two rapid Arrow presses
can request B then A before OCaml commits either. Suppressing the second request
would incorrectly leave the application at B. The group retains immediate native
navigation state separately from OCaml's selected value; it does not optimistically
mutate committed selection. An application applies requests to its current model.
This difference is required by asynchronous bridge delivery. The distinct
standalone `View.radio` intentionally suppresses checked activation and leaves
exclusivity to the application; neither API promises the other’s navigation model.

Metadata checks inspect the production semantic builder; TestPlatform navigation
checks exercise production key dispatch and the queued bridge events. Actual
macOS assistive-technology output remains a separate acceptance requirement.

Local validation for this metadata change (2026-10-01):

- `GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib`
  passes: 479 tests, two existing skips. This includes semantic output and actual
  TestPlatform Down/Down dispatch yielding B/A while committed selection remains A.
- Strict native all-target Clippy with the same features and `-D warnings` passes.
- `GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @fmt examples/gallery/main.exe`
  passes. The catalog audit and whitespace check pass.
- No desktop window was opened. New radio AX output is not yet qualified on macOS;
  Linux desktop acceptance remains deferred under OCH-47.


The [button review](button-review.md) also covers the styled ButtonGroup and
ToggleGroup exported under the component button module. Their indexed/vector
callbacks differ from the base toolbar container; GPUIO uses stable-ID intents
and current-model reducers. Connected/separated group presentation, both
orientations and singleton groups now have gallery examples, and rich button
content has a public constructor/preview. Their physical geometry/AX/input
acceptance and the remaining button-family gaps are explicit in that review.

## Installed-gallery follow-up — 2026-10-05

[Current native and installed-consumer evidence](../evidence/installed-indicators-och41.md)
now records passing custom-spinner, progress and checkable-appearance/rich-label
checks on macOS. It preserves the corrected harness assumptions and original
failures. These scoped results supersede the corresponding unrun GPU/public
walkthrough statements above; broader family, performance/resource, final-source
hosted and release requirements remain separate. VoiceOver remains on hold.
