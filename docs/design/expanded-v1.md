<!-- Imported from Linear be40cbeb-09fa-4123-a16b-784456209bab on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

# Expanded GPUIO v1: general-purpose capabilities

Status: accepted by the user on 2026-09-10. This expands v1 beyond the original GPUIX component-parity baseline. It supersedes prior statements that canvas and spring/sequence animation were only post-v1 candidates. This is planned functionality, not new implementation evidence.

## Scope and ordering

Keep the existing foundation, native interaction, managed-list and agent-chat work first (milestones 01–04). Then deliver general-purpose UI/extensions (05), desktop integration and broader examples (06), and the expanded release gate (07). There is no imposed deadline. Optional platform extensions have their own milestone and never block the required release.

Required additions:

* Typed retained custom drawing: paths, shapes, text, transforms, clipping and hit regions for an interactive diagram/plot canvas.
* A statically linked Rust/GPUI component extension SDK with normal typed OCaml properties, commands and events.
* Native springs, declarative animation sequences and synchronized repeating animations, extending <issue id="ffff8207-4a11-4fb7-96f4-5d18badc91d0" href="https://linear.app/ochat/issue/OCH-12/implement-native-declarative-animations-with-interruption-and-reduced">OCH-12</issue>.
* Declarative container-size breakpoint rules evaluated in Rust.
* Desktop integration: incoming deep links, OS notifications and actions, file-manager reveal/open, application activation and document-window state.
* A small diagramming or plotting application and a separately packaged native extension used as acceptance examples.

Optional platform packages:

* macOS native window tab groups and Dock integration.
* Wayland layer-shell windows.
* Native pixel-buffer surfaces, starting with the actual supported backend after investigation; no cross-platform video promise.

Still outside required v1: dynamic plugin loading/stable binary plugin ABI, arbitrary OCaml callbacks from native layout/paint/input, unrestricted Rust API bindings, full IDE/LSP infrastructure, terminal emulation, comprehensive docking, rich-text document editing, general multimedia/3D engines, Windows/web targets, hot reload and arbitrary keyframe/shared-layout/exit-presence systems. Ordinary images/SVGs, baseline accessibility, Eio and all original accepted contracts remain required.

## Shared architectural contract

Keep Core conventions, Eio for first-party OCaml I/O, Bonsai v0.17 on stock OCaml, the versioned bridge and native ownership of synchronous interaction. Existing API sketches remain illustrative. Native OS operations stay in Rust; OCaml task/file/network orchestration follows the Eio standard.

An extension is a trusted component compiled into the application's Rust/OCaml build, not a sandboxed plugin. Component authors may write Rust; consuming applications should use normal OCaml libraries. No raw Rust references, Rust closures, GC-managed OCaml values or borrowed native pointers become serialized public handles. Use generation-checked resources, explicit destruction and schema/capability validation.

All new features must specify accessibility, focus, hit testing, event ordering, cancellation, disposal and stale-generation behavior. Declarative resources must remain bounded; changing input or unloading a component must invalidate obsolete work. Final acceptance includes repeat mount/unmount, window close, background completion and interaction while data changes.

## Native extension SDK

Define build-time registration, namespaced component/schema identities, typed property/command/event codecs and version compatibility. Generate bindings only where it reduces drift; handwritten fixtures remain required. Runtime registration is private implementation machinery, not a stable Rust binary ABI.

A reusable package must declare its pinned native compatibility, dependencies and build/link requirements, and contribute documentation and tests. Rust handles layout/paint/immediate input. OCaml receives queued semantic events and sends explicit updates. Third-party components participate in native accessibility and semantic test queries rather than drawing inaccessible anonymous surfaces.

Validate an extension from a separate example package, consuming only documented extension APIs and no private GPUIO modules. Contain Rust panics/OCaml exceptions at the existing bridge boundary; do not describe this as process isolation.

## Drawing contract

OCaml submits retained scene data and updates. Rust owns layout, drawing, clipping and hit testing; there is no OCaml paint closure. Specify coordinate spaces/units, transform composition, z-order, clip nesting, stable item identity and geometry validation. Shapes/paths/text/images reference managed resources. Start with a bounded, clearly documented supported drawing vocabulary rather than promising every GPUI paint primitive or a shader API.

Pointer/keyboard interaction should emit typed semantic events and support focus and accessible labels/actions for interactive objects. For continuous manipulation, provide bounded native drag/pan/zoom policies as needed by the example; transport observed changes without synchronous OCaml callbacks. Test transformed hit targets and accessibility representations. A custom canvas does not automatically make arbitrary application drawings accessible.

## Motion contract

Preserve <issue id="ffff8207-4a11-4fb7-96f4-5d18badc91d0" href="https://linear.app/ochat/issue/OCH-12/implement-native-declarative-animations-with-interruption-and-reduced">OCH-12</issue>'s baseline behavior and extend supported typed numeric targets with configurable springs, ordered sequences and shared-clock repeating groups. Define position/velocity continuity on retargeting, phase/lifetime ownership, pause/resume policy, limits and cleanup. Reduced motion must yield meaningful static states; completed/disposed animations must not keep windows awake. No arbitrary per-frame OCaml easing or animation callbacks.

## Container-size rules

Evaluate typed width/height breakpoint predicates against the container's assigned native size. Deterministically choose style variants or a bounded set of supplied alternative presentations; define tie-breaking and default behavior. Contents must not drive a self-referential size-selection loop.

Specify whether inactive alternative presentations remain mounted and how focus, accessibility, subscriptions and resources behave. Native branch visibility does not imply a Bonsai lifecycle transition; any lifecycle observation must be explicit and ordered. Parent-size rules must work on the same native layout cycle without querying OCaml synchronously. This does not promise arbitrary layout algorithms authored in OCaml.

## Desktop services

Document capability discovery and typed outcomes for unsupported/unavailable operations. Receiving a deep link and opening an ordinary external link are separate operations. Support startup and already-running delivery, validation/parsing, ordering before/after app readiness, and application-selected routing without executing arbitrary URL content.

Document-window behavior includes dirty/edited state and represented file path where supported, file reveal/open and activation. Reuse <issue id="52a46bbe-65d7-4ffe-8808-86f79e48a88d" href="https://linear.app/ochat/issue/OCH-15/implement-multi-window-workspaces-tabs-and-scoped-application-commands">OCH-15</issue>'s close/quit/reopen ownership and unsaved-work policy rather than creating another lifecycle system. Linux portability includes both X11 and Wayland backend validation and desktop-environment limitations.

OS notification delivery/action handling is distinct from <issue id="448e874f-dc9e-420d-9df7-0ff08dcdef49" href="https://linear.app/ochat/issue/OCH-11/implement-accessible-controls-actions-and-native-interaction">OCH-11</issue>'s in-app notification controls. Define tags/replacement, dismissal, activation/action IDs, app/window routing, closed-window behavior and permission/unavailable outcomes. Never equate submission with confirmed user-visible delivery.

## Optional platform boundaries

macOS native tabs are distinct from ordinary in-app tabs; Dock menus are distinct from a docking layout manager. Wayland layer-shell support is for compositor-specific desktop surfaces and may be unsupported on a given compositor or backend. Default builds must not depend on these packages.

The pinned GPUI surface source exposes a macOS CoreVideo CVPixelBuffer path. First investigate ownership, synchronization, pixel formats, rendering and cleanup. Linux support requires a separately verified backend mechanism. Do not require Linux users to build macOS-only dependencies; no fallback claim without evidence. This package is not a media decoder/player, capture stack or 3D renderer.

## Acceptance and release gate

Deliver the original agent-chat example first, then an interactive diagram/plot example and a separately packaged native component. The latter examples validate drawing, event/focus/accessibility behavior, custom extension consumption, responsive container rules and richer motion. The chat or desktop example validates deep-link/document/notification workflows.

For all required additions, record tests on macOS and Linux (X11 and Wayland where relevant), semantic behavior, unavailable-capability outcomes, build versus real OS evidence, workload/hardware and resource budgets. Reuse expect/property/fixture tests and deterministic clocks. Supplement these with real OS behavior; synthetic callbacks alone are insufficient.

<issue id="8afec95a-ba31-426f-b8fb-78dfea2c5447" href="https://linear.app/ochat/issue/OCH-17/complete-component-parity-platform-validation-and-v1-distribution">OCH-17</issue> must require completion of the new required issues and broaden its parity/acceptance ledger beyond GPUIX. Optional packages may ship independently and must publish their tested support matrix and limitations. Do not let optional-ticket completion become an implicit v1 release dependency.

## Evidence and entry points

The source review used unmodified GPUI at commit a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b. Presence of a Rust API is evidence of an integration point, not proof of GPUIO support or consistent behavior on both operating systems.

* [Element lifecycle/custom implementation](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/element.rs>)
* [Canvas](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/elements/canvas.rs>) and [painting/window APIs](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/window.rs>)
* [Animation](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/elements/animation.rs>) and [springs](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/spring.rs>)
* [Container query](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/elements/container_query.rs>)
* [Application/OS integration](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/app.rs>) and [platform/window kinds](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/platform.rs>)
* [Native surface](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/elements/surface.rs>)
* [Required OCaml engineering standards](<https://linear.app/ochat/document/gpuio-ocaml-engineering-standards-core-eio-and-jane-street-conventions-38f45d0fbed5>)
* [Agent handoff](<https://linear.app/ochat/document/gpuio-start-here-agent-handoff-and-implementation-entry-point-8ecc8c41607c>)

## Ticket map

| Ticket | Capability | Release status |
| -- | -- | -- |
| [OCH-23](<https://linear.app/ochat/issue/OCH-23/implement-the-statically-linked-native-component-extension-sdk>) | Implement the statically linked native component extension SDK | Required v1 |
| [OCH-24](<https://linear.app/ochat/issue/OCH-24/implement-typed-retained-canvas-drawing-and-native-interaction>) | Implement typed retained canvas drawing and native interaction | Required v1 |
| [OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>) | Extend native animations with springs, sequences and synchronized repetition | Required v1 |
| [OCH-26](<https://linear.app/ochat/issue/OCH-26/implement-native-declarative-container-size-breakpoint-rules>) | Implement native declarative container-size breakpoint rules | Required v1 |
| [OCH-27](<https://linear.app/ochat/issue/OCH-27/implement-deep-links-and-desktop-applicationdocument-integration>) | Implement deep links and desktop application/document integration | Required v1 |
| [OCH-28](<https://linear.app/ochat/issue/OCH-28/implement-capability-aware-os-notifications-and-action-routing>) | Implement capability-aware OS notifications and action routing | Required v1 |
| [OCH-29](<https://linear.app/ochat/issue/OCH-29/validate-expanded-v1-with-a-graphics-application-and-independently>) | Validate expanded v1 with a graphics application and independently packaged native component | Required v1 |
| [OCH-30](<https://linear.app/ochat/issue/OCH-30/optional-add-macos-native-window-tabs-and-dock-integration>) | [Optional] Add macOS native window tabs and Dock integration | Optional platform package |
| [OCH-31](<https://linear.app/ochat/issue/OCH-31/optional-add-a-wayland-layer-shell-window-capability-package>) | [Optional] Add a Wayland layer-shell window capability package | Optional platform package |
| [OCH-32](<https://linear.app/ochat/issue/OCH-32/optional-investigate-and-expose-supported-native-pixel-buffer-surfaces>) | [Optional] Investigate and expose supported native pixel-buffer surfaces | Optional platform package |

<issue id="8afec95a-ba31-426f-b8fb-78dfea2c5447" href="https://linear.app/ochat/issue/OCH-17/complete-component-parity-platform-validation-and-v1-distribution">OCH-17</issue> gates all seven required additions; optional tickets are excluded from its dependency closure. <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue> remains the first setup task. No implementation was performed by this planning update.

