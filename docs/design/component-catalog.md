# GPUIO component expansion plan

Owner requested implementation tickets on 2026-09-13. This is a staged expansion of the reusable component catalog. It preserves milestones 01–04 and does not enlarge the current milestone 02 implementation goal.

## Source and evidence

GPUI itself supplies rendering, layout, input, actions, windows and platform services. The higher-level catalog evaluated here is Longbridge's GPUI Kit, principally its unstyled gpui-base layer and selected gpui-component widgets.

Pinned source: [GPUI Base exports](https://github.com/longbridge/gpui-kit/blob/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/base/src/lib.rs), [styled component exports](https://github.com/longbridge/gpui-kit/blob/84f57fdfcb4910623fb0bb7f795b077e249f9271/crates/component/src/lib.rs). Both were inspected from the local checkout of this exact revision. The source is Apache-2.0. Upstream uses gpui-pre packages; our existing Zed GPUI pin is a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b.

OCH-10 evaluation: the adapted base crate compiled against our existing GPUI pin; 154 upstream input-engine tests passed before the GPUIO bridge changes and two new bridge tests passed afterward. This does not establish compatibility of the entire styled component crate or native OS IME/accessibility acceptance. OCH-10's vendor reconstruction, manifest, license and patch records are the implementation starting point when merged. No task may depend on an author's scratch checkout.

In particular, base Table is structural table composition; the virtualized DataTable with TableDelegate lives in gpui-component. Code-editor overlays/highlighting/LSP integration also have styled-layer and additional dependency requirements. Check these independently; do not infer they work from base compilation.

## Adapter contract

Prefer reusable native behavior with GPUIO's existing typed style/theme API. Small presentation widgets may be ordinary OCaml View compositions; a wrapper around every upstream Rust struct is unnecessary. Give public user-facing component families functional equivalents, including their relevant configuration, state, commands and events; internal helpers, private types and JavaScript extension/development tooling are not separate public widgets.

Draft .mli interfaces first. Keep stock OCaml 5.3/Bonsai v0.17/Core/Eio and project engineering standards. No mandatory Async, OxCaml, JavaScript runtime or global toolchain changes.

Rust owns immediate input, focus, selection, composition, popup positioning, layout, paint and animation. OCaml owns application data and semantic decisions. Use generation-checked native leases, revisioned resources, typed queued events and explicit commands. Repeated observations must not reset native state. No synchronous OCaml rendering/delegate/formatter callback from Rust; retained data/templates and bounded asynchronous range requests bridge those APIs.

Every stateful adapter specifies controlled values versus native interaction state, identity, replacement/disposal, stale-event rejection, event ordering/coalescing, command completion, queue/payload/cache limits, keyboard behavior and accessibility. Reuse OCH-10/11/13/15 contracts instead of adding parallel controller, popup, paging or window systems. Evaluate upstream limitations honestly and isolate required patches with reproducible provenance.

## Existing ownership

| Capability | Existing ticket/milestone |
| --- | --- |
| Text input, textarea/composer, IME, undo and selection | OCH-10 / 02 |
| Buttons, checkbox/radio/toggle/switch, select/combobox, tooltip/popover/dialog, menus, command palette, progress/toasts, images/SVG, focus and accessibility | OCH-11 / 02 |
| Baseline native motion and richer springs/sequences | OCH-12 / 02; OCH-25 / 05 |
| Scrollbars/managed virtual lists, paging and explicit retention | OCH-13 / 03 |
| Markdown, highlighted code display, diff and text selection | OCH-14 / 04 |
| Windows, ordinary tabs, split/resizable panes | OCH-15 / 04 |
| Attachment/tool-result/message compositions in the chat application | OCH-16 / 04; reusable presentation API can follow in 05 |
| Native component SDK, canvas and responsive container rules | OCH-23/24/26 / 05 |
| Desktop services and OS notifications | OCH-27/28 / 06 |

## New work placement

Milestone 05 adds reusable presentation/feedback widgets, numeric/OTP/range inputs, calendars/date pickers, color pickers, disclosure/navigation components, managed trees and read-only virtualized data tables. Milestone 06 adds reusable charts/plots atop evaluated native rendering. Milestone 07 adds a public component gallery and complete coverage ledger, which distinguishes implemented, functional equivalent, deferred and unsupported capability status.

A separate milestone 08 holds editable data grids/tree editing, full docking, a native code editor and its separately optional language-service package. These were outside required v1 and remain outside the OCH-17 release dependency closure. A full IDE, terminal, rich-text document editor, multimedia engine or dynamic plugin runtime is not implied by catalog support.

## Validation

For each family: meaningful OCaml expect tests, paired codecs where wire schema changes, native interaction and stale-resource tests, bounded full traversal/update/teardown workloads, example documentation and theme/scale checks. macOS is the current native functional gate; Linux builds/unit tests are required during implementation. Record X11 and Wayland GUI/IME/accessibility coverage separately; OCH-17 remains the v1 Linux functional release gate. Post-v1 packages need their own equivalent platform evidence. Compilation and upstream mock tests are not OS behavior acceptance.

The new release catalog audit must cover the pinned public family exports, including families whose examples are implemented through existing tickets. Catalog completeness does not mean importing every transitive dependency or matching Rust spelling. Changes to upstream pins require an explicit catalog/compatibility review.


## Implementation ticket map

| Ticket | Capability | Milestone |
| --- | --- | --- |
| [OCH-33](https://linear.app/ochat/issue/OCH-33/implement-reusable-presentation-feedback-and-form-layout-components) | Implement reusable presentation, feedback and form-layout components | 05 — General-purpose UI and extensions |
| [OCH-34](https://linear.app/ochat/issue/OCH-34/implement-numeric-range-stepper-and-segmented-otp-inputs) | Implement numeric, range, stepper and segmented OTP inputs | 05 — General-purpose UI and extensions |
| [OCH-35](https://linear.app/ochat/issue/OCH-35/implement-calendar-and-date-picker-components-with-explicit-date) | Implement calendar and date-picker components with explicit date semantics | 05 — General-purpose UI and extensions |
| [OCH-36](https://linear.app/ochat/issue/OCH-36/implement-color-picker-swatches-and-color-channel-controls) | Implement color picker, swatches and color-channel controls | 05 — General-purpose UI and extensions |
| [OCH-37](https://linear.app/ochat/issue/OCH-37/implement-disclosure-navigation-and-supplementary-overlay-components) | Implement disclosure, navigation and supplementary overlay components | 05 — General-purpose UI and extensions |
| [OCH-38](https://linear.app/ochat/issue/OCH-38/implement-managed-tree-views-with-stable-identity-and-lazy-loading) | Implement managed tree views with stable identity and lazy loading | 05 — General-purpose UI and extensions |
| [OCH-39](https://linear.app/ochat/issue/OCH-39/implement-virtualized-read-only-data-tables-with-native-column) | Implement virtualized read-only data tables with native column interaction | 05 — General-purpose UI and extensions |
| [OCH-40](https://linear.app/ochat/issue/OCH-40/implement-typed-native-charts-and-reusable-plotting-components) | Implement typed native charts and reusable plotting components | 06 — Desktop integration and broader examples |
| [OCH-41](https://linear.app/ochat/issue/OCH-41/publish-the-component-gallery-and-complete-the-pinned-catalog-coverage) | Publish the component gallery and complete the pinned catalog coverage ledger | 07 — Expanded v1 validation and release |
| [OCH-42](https://linear.app/ochat/issue/OCH-42/implement-editable-data-grids-and-inline-tree-editing) | Implement editable data grids and inline tree editing | 08 — Advanced component library (post-v1) |
| [OCH-43](https://linear.app/ochat/issue/OCH-43/implement-persistent-docking-layouts-and-detachable-native-panels) | Implement persistent docking layouts and detachable native panels | 08 — Advanced component library (post-v1) |
| [OCH-44](https://linear.app/ochat/issue/OCH-44/implement-a-reusable-native-code-editor-component-and-document-model) | Implement a reusable native code-editor component and document model | 08 — Advanced component library (post-v1) |
| [OCH-45](https://linear.app/ochat/issue/OCH-45/implement-optional-eio-language-service-integration-for-the-code) | Implement optional Eio language-service integration for the code editor | 08 — Advanced component library (post-v1) |

OCH-41 blocks OCH-17 and depends on the new v1 component tickets plus the existing baseline controls/content. OCH-42–45 do not block OCH-41/OCH-17. Milestone 05 component additions depend on OCH-16 so the existing agent-chat path remains first.
