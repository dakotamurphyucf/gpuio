<!-- Imported from Linear cd517382-aa11-4313-91b2-e1ea44249efe on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

## Accepted expanded v1 scope — 2026-09-10

The user expanded v1 beyond GPUIX component parity. Read [Expanded v1 scope and contracts](<https://linear.app/ochat/document/gpuio-expanded-v1-scope-drawing-extensions-motion-and-desktop-services-46c91a3056cd>). Required: static native component SDK ([OCH-23](<https://linear.app/ochat/issue/OCH-23/implement-the-statically-linked-native-component-extension-sdk>)), retained custom drawing ([OCH-24](<https://linear.app/ochat/issue/OCH-24/implement-typed-retained-canvas-drawing-and-native-interaction>)), springs/sequences/synchronized animation ([OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>)), native declarative container rules ([OCH-26](<https://linear.app/ochat/issue/OCH-26/implement-native-declarative-container-size-breakpoint-rules>)), desktop/deep-link/document integration ([OCH-27](<https://linear.app/ochat/issue/OCH-27/implement-deep-links-and-desktop-applicationdocument-integration>)), OS notifications/actions ([OCH-28](<https://linear.app/ochat/issue/OCH-28/implement-capability-aware-os-notifications-and-action-routing>)) and a broader graphics/extension acceptance example ([OCH-29](<https://linear.app/ochat/issue/OCH-29/validate-expanded-v1-with-a-graphics-application-and-independently>)).

Preserve milestones 01–04 and the working agent-chat app first. Milestone 05 adds general-purpose UI/extensions; 06 adds desktop integration/examples; 07 is the expanded release gate. Optional platform packages—macOS native tabs/Dock ([OCH-30](<https://linear.app/ochat/issue/OCH-30/optional-add-macos-native-window-tabs-and-dock-integration>)), Wayland layer-shell ([OCH-31](<https://linear.app/ochat/issue/OCH-31/optional-add-a-wayland-layer-shell-window-capability-package>)) and native surface investigation ([OCH-32](<https://linear.app/ochat/issue/OCH-32/optional-investigate-and-expose-supported-native-pixel-buffer-surfaces>))—are tracked separately and do not block v1. Dynamic plugins/binary ABI, arbitrary synchronous OCaml layout/paint callbacks, full IDE/LSP, terminal, full docking and general multimedia engines remain outside required v1. No deadline was imposed. Earlier canvas/spring deferrals are superseded; historical evidence is unchanged.

## Required OCaml engineering baseline — 2026-09-10

Read the [OCaml engineering standards](<https://linear.app/ochat/document/gpuio-ocaml-engineering-standards-core-eio-and-jane-street-conventions-38f45d0fbed5>) before scaffolding or implementing OCaml code. The user requires Core, Eio for first-party OCaml I/O, Jane Street formatting/PPX and expect-first tests, following Ochat's conventions. The guide includes module/type design, principal `t` and receiver-first APIs, abstract representations and validated decoding, typed comparison, and `if`/pattern-match rules with RWO sources. Eio supersedes the older optional-runtime wording; pure data libraries can remain Eio-free. Historical experiments and the immutable evidence archive predate this standards update. Import these conventions into the new repository under <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue>; <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue>/20/22 implement dependencies/tooling/CI and <issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue>/9 apply API/runtime contracts.

Accepted architectural direction and staged plan. API spelling and performance budgets remain design work; proposed features are not implemented capabilities.

Published to GPUIO on 2026-09-10. [Download the source/evidence bundle](<https://uploads.linear.app/698151a6-07bd-4043-9a7b-15f84b8c23da/0a5f6f70-b343-4265-a61f-2aa405aeb710/0c24fc23-9d1e-4772-bf2b-9c2d5d9f2a05?signature=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJwYXRoIjoiLzY5ODE1MWE2LTA3YmQtNDA0My05YTdiLTE1Zjg0YjhjMjNkYS8wYTVmNmY3MC1iMzQzLTQyNjUtYTYxZi0yYWE0MDVhZWI3MTAvMGMyNGZjMjMtOWQxZS00NzcyLWJmMmItOWMyZDVkOWYyYTA1IiwiaWF0IjoxNzg5MDc2OTU5LCJleHAiOjE3ODkwNzcyNTl9.7C68RD7iAvREzifzBDfzKKRowj8GvkNU33v4OKMFWZ0>) for complete experiment sources, patches, original logs and file checksums. Historical local paths identify archive files; they are not setup instructions for a new machine.

## notes/api-design/proposal.md

# GPUIO API and v1 design

Status: high-level direction accepted by the user, 2026-09-10. View/style vocabulary, editor ownership, managed-list lifetimes and the staged delivery approach are agreed. Exact API names/signatures and implementation choices remain design work; these are not implemented capabilities. The existing experiment validates the basic Bonsai/Rust/GPUI integration on macOS only. The user has selected macOS and Linux together for v1; Windows is out of scope.

## Product and compatibility contract

Build an OCaml native application library capable of shipping a polished agent chat application: a searchable conversation sidebar, tabbed and multi-window workspaces, long transcripts, concurrent streamed responses, Markdown/code/tool results, reliable composition and editing, attachments, keyboard navigation and accessible controls.

Use stock OCaml 5.3 and Bonsai v0.17 with the documented small compatibility/native-packaging fork. Keep GPUI and Rust dependencies pinned. Use the installed versions validated in native-v017 as the initial lockfile input, rather than silently upgrading the existing project's switch. The production project gets its own reproducible dependency definition. A clean source build and packaged application on both target OSes are release requirements.

Match GPUIX's implemented desktop UI capabilities at pinned commit `18e695ed0ee8121a7793413ca795e08eda2a13df`; maintain an explicit parity ledger rather than following an ever-changing branch. Parity means available components and application-building functionality. Development-lifecycle parity is explicitly excluded: hot reload, React Refresh and native-code reload are not v1 requirements. Behavioral coverage matters more than identical JSX names or CSS strings. Multiple windows and app-declared menus go beyond that baseline. Canvas is marked planned in the pinned GPUIX baseline; GPUIO now explicitly requires it as an additional capability rather than inherited parity.

GPUI is a rendering/application framework. A full Markdown renderer, diff viewer, text editor, or select control involves additional library behavior; binding GPUI does not automatically provide Zed's editor features. The API should be extensible to more GPUI capabilities without promising every Rust API in v1.

## OCaml API shape

Use ordinary modules, labelled arguments, typed records/variants and Bonsai.Cont. No new JSX-like PPX is necessary for v1. Keep the first public surface compact:

| Layer | Responsibility |
| -- | -- |
| `Gpuio` | Typed view descriptions, styles, themes, assets, document sources, events/actions, opaque handles, application/window commands. No Bonsai or Async dependency. |
| `Gpuio_bonsai` | Bonsai driver adapter, callback registrations, managed input/list components, reusable controls and their lifecycles. |
| `Gpuio_eio` | Standard Eio I/O/task integration: capability and cancellation scopes, completion delivery, scheduling and stream coalescing. Pure data libraries may remain Eio-free. |
| `Gpuio_test` | Headless protocol/reconciliation checks and real native automation, semantic queries, deterministic clocks and diagnostics. |

These are logical module/library boundaries, not a requirement for four independently versioned repositories. One repository and one release train initially are simpler.

Basic layout constructors (`View.row`, `View.column`, `View.grid`, `Text.view`, `Image.view`) return immutable `View.t`. A Bonsai component returns `View.t Bonsai.Cont.t`. Stateful widgets such as `Text_input.create` allocate a stable controller once in the Bonsai graph. Recomputing a view reuses that controller; it must not reset editing or manufacture new native identity.

Callbacks are normal typed OCaml functions returning Bonsai effects, for example `on_press : unit Effect.t` or `on_change : Text_input.Change.t -> unit Effect.t`. Node IDs, event IDs, protocol revisions and callback lifetimes stay internal. A keyed collection requires a stable application key, not an array index. Reconciliation determines identity from parent identity, key and element kind. A kind change replaces the native widget explicitly.

Expose a convenient styled control set with accessible defaults: Button, Icon_button, Checkbox, Switch, Radio_group, Text_input, Select, Combobox, Tooltip, Dialog, Popover, Menu, Tabs, Split_pane, Progress, and notifications. Also expose composable primitives and style overrides. Rust should hold the immediate interaction state; OCaml should express options, application value, command handlers and presentation. Do not implement a second competing keyboard/focus state machine in Bonsai.

### Styling

Use a typed list of refinements with predictable last-property-wins composition. Examples of the proposed vocabulary:

```ocaml
Style.[
  padding (Px 12.);
  gap (Px 8.);
  grow 1.;
  min_height (Px 0.);
  background (Token Surface);
  radius (Px 8.);
  hover [ background (Token Surface_hover) ];
]
```

Typed lengths distinguish logical pixels, percentages, content sizing and auto where supported. Color constructors support parsed literals and semantic theme tokens; colors, shadows and gradients have structured representations. Layout exposes flex and grid, constraints, spacing, positioning, clipping, per-axis scrolling and cursors. Text styles cover fonts, weights, size, leading, wrapping, truncation, decoration, selection and syntax colors. Hover/pressed/focus styling and transitions execute natively without an OCaml round trip.

Avoid promising browser CSS semantics. In particular, define text-style inheritance, per-widget defaults, theme/local-style precedence, unsupported units, and nested scrolling explicitly. Invalid string inputs should return an error, not silently disappear. Prefer ordinary `row`/`column` layout helpers so users do not have to remember to turn on flex layout.

Theme changes should update shared tokens and invalidate affected native content; do not force applications to resend every node just to change a palette. Support light/dark/system appearance and text-size changes. The style inventory is in `gpuix-inventory.json`; every supported field needs a mapping or documented equivalent and an observable test.

## Native animations

Animation support is an explicit v1 workstream, tracked in <issue id="ffff8207-4a11-4fb7-96f4-5d18badc91d0" href="https://linear.app/ochat/issue/OCH-12/implement-native-declarative-animations-with-interruption-and-reduced">OCH-12</issue>. OCaml declares typed target values and timing once; Rust computes intermediate values and schedules GPUI frames. No per-frame Bonsai/FFI update is required. Cover width/height, positional offsets, opacity and corner radius with duration, delay, standard easing and cubic Bezier timing. A changed target interrupts from the currently visible value, avoiding jumps.

Also support basic repeating loading/streaming indicators, lifecycle cancellation, reduced-motion policy and deterministic clock tests. Finished/disposed/hidden animations must not keep windows redrawing. Define optional completion/cancellation events once per generation. Native animation still has a layout/paint cost: reveal a sidebar with an outer clipping container when possible instead of reflowing its inner text every frame.

GPUIX's implemented motion API does not include springs, arbitrary keyframes, exit-presence orchestration or shared-layout transitions. The pinned GPUI source itself exposes repeating, chained and spring animation facilities plus reduced-motion behavior. The accepted expansion requires springs, declarative sequences and synchronized repetition in [OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>), beyond the original parity baseline. This is planned coverage, not an implemented GPUIO animation feature.

## Virtual lists and infinite history

Expose both a simple retained-child `Virtual_list.view` and a managed Bonsai `Virtual_list.component`. The first mirrors GPUIX's basic list: retain all descriptions but construct/layout/paint only visible rows. The second builds OCaml and Rust row views only for the viewport plus overscan, focused/composing rows and any explicitly pinned rows.

The managed component accepts a keyed collection/data source, a reactive per-row renderer, estimated or fixed height, and scroll policy. Offer an in-memory collection adapter first and a paged source adapter with load-before/load-after, end-of-history, loading/error/retry and explicit cancellation. Infinite loading and render virtualization are distinct features, even if presented by one convenience component.

Native GPUI ListState owns measurements and scrolling. It requests missing ranges asynchronously; its synchronous render callback uses available row descriptions or measured/estimated placeholders and never calls OCaml. Requests carry list/data generations so a delayed response cannot populate a different conversation. Update the requested range on collection changes, resize, focus and programmatic scroll as well as wheel events.

Preserve the visible anchor as stable row key plus offset when older history is prepended. Invalidate only affected heights when text streams, an image loads, the font/theme changes or the viewport width changes. Follow the tail only while the user remains at the bottom; scrolling up pauses following. A jump-to-latest command resumes it. Apply programmatic scrolling after the same transaction's list changes are installed.

There are three separate lifetimes: application record, Bonsai row computation and mounted native view. Offscreen is not deleted. Bonsai v0.17 retains non-default keyed models after deactivation, so merely shrinking an assoc map does not prove bounded memory. The managed list must define an explicit policy for resetting/evicting transient row state; persistent expansion/draft state belongs in a keyed application store. Focus/composition/selection must pin required resources until completion. Test memory after visiting the entire history, not just after opening the first screen.

Row height metadata can still be O(logical row count), and stored conversation data can be O(history). The promised bound is on active views and expensive native caches, not on all application memory. Very large/unbounded data needs paging and metadata compaction. A single huge Markdown row also needs block/line virtualization or an explicit expand limit; row-level virtualization alone does not solve it.

## Text editing, streaming and rich content

`Text_input` should expose an editor controller with an observed value and explicit commands to focus, select, clear or replace. User edits update the native buffer immediately. Selection, caret, IME composition, undo/redo, grapheme movement and clipboard actions stay in Rust. A normal value observation is not a command to overwrite the native editor. Programmatic replacements specify whether to preserve/reset selection and undo history, and use revision checks internally.

Declare submit policy natively: Enter submits only outside IME composition, Shift+Enter inserts a newline, and platform shortcut alternatives are available. Submit events include the native text snapshot being submitted so an OCaml observation lag cannot send stale text. External validation observes edits and sends explicit corrections; it cannot synchronously veto an already completed native edit. Large editable documents can later opt into delta events rather than full value notifications.

Represent long display content with a stable, versioned `Text_source.t`. Provide `of_string` for simple use and append/replace operations for streams. OCaml owns canonical conversation content; Rust retains the necessary display copy. A streamed update sends an append/edit to that document resource, not the transcript tree or the entire accumulated response on every token. Coalesce arriving chunks up to the next display opportunity and preserve the final completion/cancellation state. Memory and pending work must remain bounded if rendering falls behind.

Rust Markdown/Code/Diff widgets own parsing, layout, selection and painting. Start with a mature parser and bounded background parsing/highlighting for the changed document; display plain or provisional content while syntax colors catch up. Jobs are keyed by document revision and stale results are discarded. Cache unchanged documents and reusable blocks. Do not claim that append-only transport makes Markdown parsing incremental: incomplete fences, reference definitions and other syntax can invalidate earlier interpretation. More elaborate incremental parsing needs a correctness model and measurements before adding it.

Markdown v1 should cover paragraphs, headings, lists/task lists, quotes, links, tables, fenced code, inline formatting, images and selection. Code should support language/path selection, line numbers, copy, search highlights and horizontal scrolling. Prioritize OCaml as well as Rust, JSON, shell, Python, TypeScript/JavaScript and common configuration formats. Diff should provide file/line navigation, collapse/expand, additions/deletions, word highlighting and a virtualized standalone mode. Tool calls remain ordinary OCaml-composed cards containing these widgets.

Define selection using stable content IDs/ranges rather than temporary row indices. Scrolling a selected row away must not silently corrupt the selection. Copying across unloaded rows requires a document/data-source operation; keep that distinct from copying the current native selection. Accessibility should announce meaningful completed chunks or status changes, not every streamed token.

## Windows, commands and native capabilities

One process owns one GPUI application and multiple independently scoped windows. Start with one OCaml UI domain owning all Bonsai drivers; a driver per window shares application state through explicit stores/events. Do not share a Bonsai graph concurrently across domains. Run IO and CPU work outside UI critical sections and deliver completions through the runtime adapter.

Declare an initial window and open/close subsequent windows with effects. Opaque typed Window/Focus/Scroll/Editor handles include internal generation checks. Closing a window cancels its tasks and drops its native resources while other windows remain usable. Application-wide work may explicitly outlive a window. Define last-window-close, reopen, quit and unsaved-work close confirmation for each OS. An asynchronous close decision keeps the window alive until resolved; it does not block an OS callback waiting on OCaml.

Tabs are application controls, not assumed operating-system tab groups. v1 supports keyboard tab navigation, per-tab state/scroll retention and opening a conversation in another window. Drag-to-detach and full docking can follow later unless the reference application establishes a need.

Use a typed action/command registry shared by buttons, keyboard shortcuts, menus and a command palette. Bind matching, enabled state, focus routing, default handling and propagation rules natively. OCaml receives action notifications; arbitrary late callbacks cannot implement synchronous preventDefault. Provide typed focus scopes, dialog traps, pointer capture, drag/drop, clipboard, file dialogs, URL opening, theme/scale changes and window bounds/chrome. Use a capability query plus explicit Unsupported results for platform differences; do not pretend macOS and Linux desktop facilities are identical.

Accessibility roles, names, values, states and actions belong in the view model from the first controls. Use GPUI's AccessKit support and test the actual macOS/Linux bridges. A role field alone is not accessibility parity: keyboard operation, focus, virtualized content and rich-text semantics need behavioral checks.

## Rust interface and performance model

Use two Rust crates initially: a pure `gpuio_protocol` and `gpuio_native`. Keep FFI, transport, retained tree, GPUI adaptation, widgets and platform operations as modules within the native crate. The protocol contains owned portable data, not GPUI types. Avoid a public ABI exposing Rust ownership or raw GPUI objects.

The FFI surface stays small: initialize/run/shutdown, submit a batch, drain events and receive a wakeup. Use the already tested ocaml-interop/bin_prot approach with explicit ownership of buffers and a handshake for protocol versions/capabilities. Treat variant tags/schema evolution as a deliberate contract with cross-language fixtures; OCaml variant reordering must not accidentally change the wire meaning.

Transactions contain per-window revisions and operations such as create/remove, update styles/properties, child splices, event bindings, document appends/edits and commands. Validate IDs, bounds, byte/count limits and revision expectations before mutation; commit atomically. Return structured errors. Queue requests that need a result with a correlation ID. Never synchronously enter OCaml from GPUI paint/layout/input callbacks.

Keep one in-flight tree transaction per window initially. Reconcile the newest desired state against the acknowledged state; do not queue obsolete full trees. Track transaction acceptance separately from rendering completion and logical no-change cycles. The v0.17 lifecycle fixes remain part of this contract. Replace the experimental permanent 60 Hz timer with wakeups from actions, work completion, time deadlines and requested animation frames before v1; clocks/after-display subscriptions still need explicit scheduling even when pixels do not change.

Use a retained arena with generation-checked IDs and parent links, plus native widget entities for persistent state. Build ephemeral GPUI elements during render using shared immutable strings/documents. Mark affected ancestors/list rows dirty. Atomicity need not require copying the whole retained tree each commit: validate into a patch plan and apply only touched records with a rollback/journal or equivalent design.

Bound/coalesce queues by event semantics. Viewport/mouse-position observations may be replaceable; button edges, submit, composition transitions and completion/cancellation events must preserve ordering. Text observation coalescing must preserve its latest revision and the exact snapshot used by submit. Never block the OS input callback on a full queue; implement explicit overload handling and test it rather than silently dropping semantic events.

Do not scan/serialize all history on each token. Use keyed collection splices and document revisions to identify changes. Start with straightforward reconciliation for ordinary trees, instrument it, then add caches only at measured hot spots. An extension path may register additional Rust widget adapters against typed protocol schemas, but v1 should not require a dynamic plugin ABI or generic serialization of arbitrary GPUI objects. Custom drawing can later use a retained drawing-command resource.

Before implementing a large editor/control stack, evaluate reusable GPUI widget implementations, including Longbridge's gpui-component/gpui-base ecosystem (now under gpui-kit). Adoption must pass compatibility with our pinned GPUI, native-owned interaction and theme contracts. This is an evaluation item, not a selected dependency or a reason to replace the working bridge wholesale.

## Iterative delivery

Every milestone adds a usable vertical slice to the same reference application on macOS and Linux. Do not finish the entire OCaml layer before wiring Rust, or finish macOS before starting Linux.

| Milestone | Deliverable | Exit evidence |
| -- | -- | -- |
| 1. Reproducible foundation | Real repository, pinned dependency/fork recipe, minimal public view API, protocol revisions, application/window identity; macOS and Linux builds | Clean builds and a window/input smoke test on both OSes, shutdown/error tests, first two-window skeleton |
| 2. Native interaction | Layout/theme, accessible controls, native editor, actions/focus, basic assets/menus/dialogs | IME/selection/undo/clipboard and keyboard tests; example composer and settings panel |
| 3. Long conversations | Retained and managed lists, history paging, anchors/tail behavior, bounded caches and revisioned streaming | Large synthetic transcript, prepends and changing heights, scrolling away/returning, stalled-consumer and memory tests |
| 4. Agent workspace | Markdown/code/diff, attachments and tool cards, search, tabs, complete multi-window lifecycle | Multiple streams while editing; independent windows; close/cancel and restore-focus tests |
| 5. Parity and release | Remaining implemented GPUIX UI/style/event coverage, native animation, diagnostics/automation, docs, packaging | Completed parity ledger and documented gaps; install/run packaged app on clean macOS/Linux systems |

Performance gates are proposed targets, not observed results: test 10,000 variable-height loaded records, a 100,000-logical-row paged source, a growing large response, and several simultaneous synthetic streams while typing/scrolling. Record UI-domain work, mutation bytes, parser time, queue depth, frame latency and memory separately. Aim for p95 frame work within a 60 Hz frame budget on named reference machines; record end-to-end input/paint latency separately. Check memory after repeated full-history traversal and window close, and verify idle applications do not continuously redraw. Hardware, document-size limits and acceptable cache budgets must be fixed before these become pass/fail release thresholds.

Linux validation must cover both Wayland and X11, real text input/clipboard/file-drop paths, display scaling and accessibility. CI builds/headless tests are necessary but do not substitute for compositor/IME/screen-reader validation. Choose and publish minimum OS/library versions and architecture artifacts during milestone 1; the current macOS-only experiment does not determine those automatically.

A viable v1 includes semantic controls, style coverage, virtualized/paged lists, editor-quality chat composition, streamed rich content, tabs and multiple windows, keyboard/accessibility support, test automation, and documented distributable builds. It is a UI library: LLM vendors, agent orchestration, persistence schema and credential management belong to the reference application or separate integrations. Provide a deterministic fake streaming backend so UI tests require no external provider.

Do not make full Zed editing/LSP, arbitrary Rust API bindings, terminal emulation, web targets, dockable IDE workspaces or native-code hot reload release blockers. Development-lifecycle parity is not required. Hot reload, React Refresh, native-code reload and preserving state across code changes are not release requirements. Normal reproducible builds, useful diagnostics, documentation and packaging remain necessary for a usable library. An embedded updater is optional platform integration, not a blocker for component/application-capability parity.

## Accepted contracts and remaining implementation decisions

The user accepted the view/style vocabulary, editor ownership contract, managed-list state semantics and staged implementation recommendations. Persistent application data is distinct from disposable row presentation, with explicit retention for active editing/selection. A live editor is native-owned; its observations do not overwrite newer edits. Submission includes the exact native snapshot, and delayed clear/replacement must not erase subsequent typing.

Next implementation work should settle exact module signatures, style inheritance/unset semantics, editor/control reuse, list eviction/reset mechanics and the platform/architecture support matrix. Huge-document selection and measured cache/performance budgets need concrete tests before API freeze. These are bounded design tasks within the agreed direction, not reasons to reopen the accepted ownership model.

The expanded backlog contains 27 implementation/setup issues: 24 required items and 3 optional platform extensions, across seven required milestones plus an optional milestone. See decisions.md and linear-backlog.md for the accepted scope and issue mapping. Do not infer that any backlog capability has already shipped.

## Sources

* [GPUIX pinned README](<https://github.com/remorses/gpuix/blob/18e695ed0ee8121a7793413ca795e08eda2a13df/README.md>): implemented UI and declared omissions.
* [GPUIX host types](<https://github.com/remorses/gpuix/blob/18e695ed0ee8121a7793413ca795e08eda2a13df/packages/react/src/types/host.ts>): styles, events and component contracts.
* [GPUIX native renderer](<https://github.com/remorses/gpuix/blob/18e695ed0ee8121a7793413ca795e08eda2a13df/packages/native/src/renderer.rs>): retained rows and ListState integration.
* [GPUI ListState](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/elements/list.rs>): measurement, splices, tail following, focus and scroll state.
* [GPUI input contract](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/input.rs>): synchronous input/IME methods.
* [GPUI application API](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/app.rs>): windows, actions and platform operations.
* [GPUI accessibility](<https://github.com/zed-industries/zed/blob/a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b/crates/gpui/src/window/a11y.rs>): AccessKit architecture.
* [GPUI Kit](<https://github.com/longbridge/gpui-kit>): potential reusable behavior/widgets; not yet validated with GPUIO.
* [Validated v0.17 experiment](<https://uploads.linear.app/698151a6-07bd-4043-9a7b-15f84b8c23da/0a5f6f70-b343-4265-a61f-2aa405aeb710/0c24fc23-9d1e-4772-bf2b-9c2d5d9f2a05?signature=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJwYXRoIjoiLzY5ODE1MWE2LTA3YmQtNDA0My05YTdiLTE1Zjg0YjhjMjNkYS8wYTVmNmY3MC1iMzQzLTQyNjUtYTYxZi0yYWE0MDVhZWI3MTAvMGMyNGZjMjMtOWQxZS00NzcyLWJmMmItOWMyZDVkOWYyYTA1IiwiaWF0IjoxNzg5MDc2OTU5LCJleHAiOjE3ODkwNzcyNTl9.7C68RD7iAvREzifzBDfzKKRowj8GvkNU33v4OKMFWZ0>) (file reference: `../native-v017-results.md`): evidence and limitations of the existing bridge.

