<!-- Imported from Linear 8d6ec703-f257-446a-855a-72c289eda050 on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

## Accepted expanded v1 scope — 2026-09-10

The user expanded v1 beyond GPUIX component parity. Read [Expanded v1 scope and contracts](<https://linear.app/ochat/document/gpuio-expanded-v1-scope-drawing-extensions-motion-and-desktop-services-46c91a3056cd>). Required: static native component SDK ([OCH-23](<https://linear.app/ochat/issue/OCH-23/implement-the-statically-linked-native-component-extension-sdk>)), retained custom drawing ([OCH-24](<https://linear.app/ochat/issue/OCH-24/implement-typed-retained-canvas-drawing-and-native-interaction>)), springs/sequences/synchronized animation ([OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>)), native declarative container rules ([OCH-26](<https://linear.app/ochat/issue/OCH-26/implement-native-declarative-container-size-breakpoint-rules>)), desktop/deep-link/document integration ([OCH-27](<https://linear.app/ochat/issue/OCH-27/implement-deep-links-and-desktop-applicationdocument-integration>)), OS notifications/actions ([OCH-28](<https://linear.app/ochat/issue/OCH-28/implement-capability-aware-os-notifications-and-action-routing>)) and a broader graphics/extension acceptance example ([OCH-29](<https://linear.app/ochat/issue/OCH-29/validate-expanded-v1-with-a-graphics-application-and-independently>)).

Preserve milestones 01–04 and the working agent-chat app first. Milestone 05 adds general-purpose UI/extensions; 06 adds desktop integration/examples; 07 is the expanded release gate. Optional platform packages—macOS native tabs/Dock ([OCH-30](<https://linear.app/ochat/issue/OCH-30/optional-add-macos-native-window-tabs-and-dock-integration>)), Wayland layer-shell ([OCH-31](<https://linear.app/ochat/issue/OCH-31/optional-add-a-wayland-layer-shell-window-capability-package>)) and native surface investigation ([OCH-32](<https://linear.app/ochat/issue/OCH-32/optional-investigate-and-expose-supported-native-pixel-buffer-surfaces>))—are tracked separately and do not block v1. Dynamic plugins/binary ABI, arbitrary synchronous OCaml layout/paint callbacks, full IDE/LSP, terminal, full docking and general multimedia engines remain outside required v1. No deadline was imposed. Earlier canvas/spring deferrals are superseded; historical evidence is unchanged.

## Required OCaml engineering baseline — 2026-09-10

Read the [OCaml engineering standards](<https://linear.app/ochat/document/gpuio-ocaml-engineering-standards-core-eio-and-jane-street-conventions-38f45d0fbed5>) before scaffolding or implementing OCaml code. The user requires Core, Eio for first-party OCaml I/O, Jane Street formatting/PPX and expect-first tests, following Ochat's conventions. The guide includes module/type design, principal `t` and receiver-first APIs, abstract representations and validated decoding, typed comparison, and `if`/pattern-match rules with RWO sources. Eio supersedes the older optional-runtime wording; pure data libraries can remain Eio-free. Historical experiments and the immutable evidence archive predate this standards update. Import these conventions into the new repository under <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue>; <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue>/20/22 implement dependencies/tooling/CI and <issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue>/9 apply API/runtime contracts.

Accepted high-level decisions. Exact API names and implementation choices remain open within the approved direction.

Published to GPUIO on 2026-09-10. [Download the source/evidence bundle](<https://uploads.linear.app/698151a6-07bd-4043-9a7b-15f84b8c23da/0a5f6f70-b343-4265-a61f-2aa405aeb710/0c24fc23-9d1e-4772-bf2b-9c2d5d9f2a05?signature=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJwYXRoIjoiLzY5ODE1MWE2LTA3YmQtNDA0My05YTdiLTE1Zjg0YjhjMjNkYS8wYTVmNmY3MC1iMzQzLTQyNjUtYTYxZi0yYWE0MDVhZWI3MTAvMGMyNGZjMjMtOWQxZS00NzcyLWJmMmItOWMyZDVkOWYyYTA1IiwiaWF0IjoxNzg5MDc2OTU4LCJleHAiOjE3ODkwNzcyNTh9.G_XnbMMgFCLecZg_082F_afh7es_W4dN7oLBh9_EVn4>) for complete experiment sources, patches, original logs and file checksums. Historical local paths identify archive files; they are not setup instructions for a new machine.

## notes/api-design/decisions.md

# Accepted GPUIO direction — 2026-09-10

The user accepted the recommended high-level API contracts and clarified that GPUIX parity means components and application-building functionality. It does not mean parity with its development lifecycle. Hot reload is explicitly not a requirement.

## Baseline and scope

* Stock OCaml, Bonsai v0.17, and the small documented compatibility/native-packaging fork; defer OxCaml.
* macOS and Linux together for v1; no Windows target.
* Reference application: a polished agent chat UI with long conversations, streaming, native input, rich text/code/diff, search, attachments, tabs and multiple windows.
* GPUIX component/style/event functionality is a pinned coverage baseline. Multiple windows and app-defined menus are additional goals. No hot reload, React Refresh or native-code reload requirement.
* Normal builds, tests, diagnostics, documentation and distributable application packaging remain release requirements. An embedded updater is optional integration, not component parity.

## View and style contract

* Ordinary OCaml modules, labelled arguments and Bonsai.Cont; no new markup syntax required.
* Layout/content/control vocabulary, typed refinements and useful accessible control defaults.
* Predictable composition, explicit units, text/theme inheritance and unset behavior.
* Native hover, pressed, focus, disabled and animation behavior; application callbacks are typed effects.
* Public handles are opaque; application keys are stable; bridge IDs/revisions remain internal.

## Editor contract

* Rust owns the live editing session: text buffer, selection, IME, undo, caret, scrolling and immediate native input behavior.
* OCaml owns application state, saved drafts, validation and send/persist decisions.
* Observations and mutation commands are distinct; an older observation cannot overwrite a newer native edit.
* Submission carries the exact native text/revision. Delayed clear/replacement must be conditional when needed to avoid erasing a newly typed draft.
* Commands explicitly define selection and undo behavior. Enter submission respects IME composition.

## Managed-list contract

* Stable keys preserve identity through reorder and history prepend.
* Separate application-record lifetime, Bonsai-row computation lifetime, native-view lifetime and cache lifetime.
* Persistent application state; disposable row presentation; explicit retention for focused/composing/selected resources.
* Offscreen is not deleted. Viewport visibility is not identical to Bonsai activation because of overscan.
* Streaming/task lifetime follows the conversation/application scope, not visibility of a row.
* Explicit eviction/reset policy for v0.17 keyed models, validated after full-history traversal.
* Paging, native measurement, scroll anchoring, tail following and stale-request generation handling are library responsibilities in the managed list.

## Delivery

Use vertical slices on both OSes: foundation, native interaction, long conversations, agent workspace, general-purpose UI/extensions, desktop integration/examples, expanded validation/release. Begin a staged implementation backlog now. Exact API names/signatures, widget reuse, concrete eviction mechanics, platform minimums and measured budgets remain implementation decisions. A task being in Linear does not mean the capability exists.

## Animation scope

The user asked specifically about animation support. Native animations are included in v1 and now have a dedicated issue, <issue id="ffff8207-4a11-4fb7-96f4-5d18badc91d0" href="https://linear.app/ochat/issue/OCH-12/implement-native-declarative-animations-with-interruption-and-reduced">OCH-12</issue>: typed target transitions, duration/delay/easing, smooth interruption, basic repeating indicators, cleanup and reduced-motion behavior. Rust advances animation frames; OCaml supplies targets and receives optional endpoint events. Springs, declarative sequences and synchronized repeating animation are now required in [OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>). Arbitrary keyframe/shared-layout/exit-presence systems remain outside required v1.

