<!-- Imported from Linear 079542ed-bad8-445c-a5a4-b8afd894c055 on 2026-09-11.
Historical paths and evidence are references, never build inputs. -->

## Accepted expanded v1 scope — 2026-09-10

The user expanded v1 beyond GPUIX component parity. Read [Expanded v1 scope and contracts](<https://linear.app/ochat/document/gpuio-expanded-v1-scope-drawing-extensions-motion-and-desktop-services-46c91a3056cd>). Required: static native component SDK ([OCH-23](<https://linear.app/ochat/issue/OCH-23/implement-the-statically-linked-native-component-extension-sdk>)), retained custom drawing ([OCH-24](<https://linear.app/ochat/issue/OCH-24/implement-typed-retained-canvas-drawing-and-native-interaction>)), springs/sequences/synchronized animation ([OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>)), native declarative container rules ([OCH-26](<https://linear.app/ochat/issue/OCH-26/implement-native-declarative-container-size-breakpoint-rules>)), desktop/deep-link/document integration ([OCH-27](<https://linear.app/ochat/issue/OCH-27/implement-deep-links-and-desktop-applicationdocument-integration>)), OS notifications/actions ([OCH-28](<https://linear.app/ochat/issue/OCH-28/implement-capability-aware-os-notifications-and-action-routing>)) and a broader graphics/extension acceptance example ([OCH-29](<https://linear.app/ochat/issue/OCH-29/validate-expanded-v1-with-a-graphics-application-and-independently>)).

Preserve milestones 01–04 and the working agent-chat app first. Milestone 05 adds general-purpose UI/extensions; 06 adds desktop integration/examples; 07 is the expanded release gate. Optional platform packages—macOS native tabs/Dock ([OCH-30](<https://linear.app/ochat/issue/OCH-30/optional-add-macos-native-window-tabs-and-dock-integration>)), Wayland layer-shell ([OCH-31](<https://linear.app/ochat/issue/OCH-31/optional-add-a-wayland-layer-shell-window-capability-package>)) and native surface investigation ([OCH-32](<https://linear.app/ochat/issue/OCH-32/optional-investigate-and-expose-supported-native-pixel-buffer-surfaces>))—are tracked separately and do not block v1. Dynamic plugins/binary ABI, arbitrary synchronous OCaml layout/paint callbacks, full IDE/LSP, terminal, full docking and general multimedia engines remain outside required v1. No deadline was imposed. Earlier canvas/spring deferrals are superseded; historical evidence is unchanged.

## Required OCaml engineering baseline — 2026-09-10

Read the [OCaml engineering standards](<https://linear.app/ochat/document/gpuio-ocaml-engineering-standards-core-eio-and-jane-street-conventions-38f45d0fbed5>) before scaffolding or implementing OCaml code. The user requires Core, Eio for first-party OCaml I/O, Jane Street formatting/PPX and expect-first tests, following Ochat's conventions. The guide includes module/type design, principal `t` and receiver-first APIs, abstract representations and validated decoding, typed comparison, and `if`/pattern-match rules with RWO sources. Eio supersedes the older optional-runtime wording; pure data libraries can remain Eio-free. Historical experiments and the immutable evidence archive predate this standards update. Import these conventions into the new repository under <issue id="efb25a56-07a3-44a9-b66c-c2d99d7108f2" href="https://linear.app/ochat/issue/OCH-18/create-the-gpuio-git-repository-and-project-scaffold">OCH-18</issue>; <issue id="8a103a3d-1eba-4b92-a2f0-04fbe66bc522" href="https://linear.app/ochat/issue/OCH-19/pin-ocaml-rust-and-gpui-dependencies-and-package-the-bonsai">OCH-19</issue>/20/22 implement dependencies/tooling/CI and <issue id="d0337cd4-62ce-4e3c-9274-fbd9703a2614" href="https://linear.app/ochat/issue/OCH-8/implement-the-typed-ocaml-view-style-and-theme-api">OCH-8</issue>/9 apply API/runtime contracts.

Illustrative API examples and pinned component/functionality coverage ledger. Examples are not compiled SDK code. Hot reload is not required.

Published to GPUIO on 2026-09-10. [Download the source/evidence bundle](<https://uploads.linear.app/698151a6-07bd-4043-9a7b-15f84b8c23da/0a5f6f70-b343-4265-a61f-2aa405aeb710/0c24fc23-9d1e-4772-bf2b-9c2d5d9f2a05?signature=eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJwYXRoIjoiLzY5ODE1MWE2LTA3YmQtNDA0My05YTdiLTE1Zjg0YjhjMjNkYS8wYTVmNmY3MC1iMzQzLTQyNjUtYTYxZi0yYWE0MDVhZWI3MTAvMGMyNGZjMjMtOWQxZS00NzcyLWJmMmItOWMyZDVkOWYyYTA1IiwiaWF0IjoxNzg5MDc2OTYwLCJleHAiOjE3ODkwNzcyNjB9.LAPkFAJcd4KD7lsWQ7DCbKSXqhwOZz500J46za3J078>) for complete experiment sources, patches, original logs and file checksums. Historical local paths identify archive files; they are not setup instructions for a new machine.

## notes/api-design/api-examples.md

# Proposed OCaml usage

These are API sketches for discussion, not code against an existing GPUIO SDK. They deliberately use ordinary OCaml and the v0.17 `Bonsai.Cont` programming model. Exact module signatures remain to be designed.

## A chat pane

In this sketch, `messages` is a reactive keyed collection, each message body is a versioned `Text_source.t`, and `send` is an OCaml callback returning a Bonsai effect. The managed list handles viewport tracking internally.

```ocaml
open Gpuio
open Gpuio_bonsai
module B = Bonsai.Cont

let chat ~messages ~send graph =
  let open B.Let_syntax in
  let editor = Text_input.create ~initial_text:"" graph in
  let transcript =
    Virtual_list.component
      (module Message.Id)
      ~items:messages
      ~height:(Variable { estimated = 180. })
      ~scroll:Follow_tail_when_at_end
      ~render:(fun message _graph ->
        let%arr message in
        Markdown.view ~source:message.body ())
      graph
  in
  let%arr editor and transcript in
  View.column
    ~style:Style.[grow 1.; min_height (Px 0.); gap (Px 12.)]
    [ transcript
    ; Text_input.view editor
        ~placeholder:"Ask anything…"
        ~multiline:(Grow { min_rows = 1; max_rows = 8 })
        ~submit:Enter_unless_composing
        ~on_submit:send
        ()
    ]
```

`Text_input.create` returns a reactive controller/snapshot with stable native identity. `on_submit` receives the native text being submitted, not a potentially stale observed value. Clearing after successful submission is an explicit effect on the editor handle. The sketch omits that policy because a real chat application may preserve the draft when sending fails.

`Variable` and `Grow` are proposed typed configuration variants. `Message.Id` supplies identity/comparison. The row renderer runs in OCaml when its active data changes; GPUI's synchronous list renderer never calls it through FFI.

## Styling a reusable button

```ocaml
Button.view
  ~label:"Stop generation"
  ~on_press:cancel_generation
  ~style:Style.[
    padding_xy ~x:(Px 12.) ~y:(Px 8.);
    radius (Px 6.);
    background (Token Danger);
    hover [background (Token Danger_hover)];
  ]
  ()
```

Button supplies role, focusability, keyboard activation and disabled behavior. Native hover/pressed visuals do not depend on an OCaml callback. A lower-level `View` plus action/focus primitives remains available for custom controls.

## Paged history and streaming

The simple collection adapter accepts a keyed in-memory sequence. A paged adapter adds `load_before`, `load_after`, stable cursor boundaries, loading/error state and cancellation. The adapter implements the same list-facing collection contract, so changing storage does not require rewriting the row renderer.

The application appends incoming chunks to a versioned document source. The runtime combines pending chunks, sends a document append command against the last accepted revision and notifies the affected native row. It does not resend the whole transcript. A user edit or regenerated answer produces an explicit replacement/new generation so delayed chunks cannot append to the wrong answer.

Expose task results as typed values/effects through the standard Eio adapter. All first-party OCaml I/O uses Eio; pure static view descriptions need not depend on Eio or an LLM SDK.

## Windows and commands

An application has initial window specifications and can later execute `Window.open_`, `Window.close`, `Window.set_title` and `Focus.request` effects on typed handles. A window's root is a Bonsai component with a per-window lifecycle scope. App-level stores can be shared explicitly across windows.

Register one typed command for an action such as `Send_message` or `New_conversation`. Buttons, platform menus, shortcut bindings and command-palette entries refer to that command. Native matching and propagation happen before the resulting action is sent to OCaml. This keeps shortcut behavior consistent and avoids exposing a misleading asynchronous `prevent_default` API.

---

## notes/api-design/parity-ledger.md

# GPUIX capability ledger and GPUIO v1 scope

Baseline: GPUIX `18e695ed0ee8121a7793413ca795e08eda2a13df`, inspected 2026-09-10. This is a planning ledger, not a claim of implementation or cross-platform verification. The user confirmed component and application-functionality parity only. Development-lifecycle parity, including hot reload, is excluded. Functional equivalents count; JSX/CSS spelling does not need to match. Proposed stages refer to proposal.md. The complete extracted style/event-name inventory is in gpuix-inventory.json.

| Capability | Baseline evidence | Proposed OCaml surface | Stage / decision |
| -- | -- | -- | -- |
| Flex/grid, sizes, spacing, positioning, clipping | README styling; host.ts StyleDesc | View.row/column/grid and typed Style refinements | 2, coverage completion 7 |
| Colors, gradients, borders, shadows, typography | host.ts StyleDesc | Typed color/length/paint values, theme tokens | 2/7 |
| Hover/active and native transitions | host.ts Motion/Style types; README animations | Style states and Animation transitions | 2/5; dedicated <issue id="ffff8207-4a11-4fb7-96f4-5d18badc91d0" href="https://linear.app/ochat/issue/OCH-12/implement-native-declarative-animations-with-interruption-and-reduced">OCH-12</issue> |
| Selectable text and formatted runs | README text selection | Text and Rich_text | 2/4 |
| Text selection across elements | README selection; native text module | Selection scopes and stable content ranges | 4/5; define unloaded-row behavior |
| Text highlighting/search | hooks/use-text-search.ts; host.ts HighlightSpec | Text_search, result/navigation effects | 4 |
| Images and tintable SVG | custom_elements/img.rs, [svg.rs](<http://svg.rs>) | Image, Icon, Asset | 2/4 |
| Native single/multiline editing | custom_elements/input.rs; README input | Text_input controller and view | 2 |
| Auto-grow, selection, IME, undo/redo, clipboard | README input; input implementation | Typed editor configuration and commands | 2; real OS tests |
| Variable-height retained list | [renderer.rs](<http://renderer.rs>) ListState integration | Virtual_list.view | 3 |
| Windowed rows via itemCount/windowStart | host.ts VirtualListProps; README performance model | Managed Virtual_list.component | 3; automatic OCaml window management adds value |
| Tail following, anchors, height invalidation | [renderer.rs](<http://renderer.rs>); README lists | Scroll policy and typed scroll commands | 3 |
| Infinite history loading | App responsibility in baseline | Collection.Paged adapter | 3; additional library feature |
| Markdown and syntax-highlighted code | native markdown/syntax modules | Markdown, Code and Text_source | 4 |
| Unified diff and collapse/navigation events | native diff modules; host.ts DiffProps | Diff viewer and typed events | 4 |
| Anchored overlays, Select, Combobox, Tooltip | components/*.tsx; [anchored.rs](<http://anchored.rs>) | Managed controls with overridable styling | 2/7 |
| Dialog focus traps and outside clicks | README focus/overlays | Dialog, Focus_scope, Popover | 2 |
| Pointer/keyboard/focus/scroll/file-drop events | README events; host.ts; native index.d.ts | Typed Event values and declarative routing | 2/7 |
| Pointer capture | README event behavior | Native capture policy on interactive primitives | 2/7 |
| Accessibility roles/states/actions | native a11y support; README accessibility | Semantic defaults plus Accessibility overrides | 2/5; macOS and Linux behavior gates |
| Native scrolling/imperative focus/bounds | native index.d.ts | Typed Scroll/Focus/Element handles and queries | 2/3 |
| Window title, chrome, insets, background launch | native index.d.ts WindowOptions | Window options and reactive observations | 1/2/7 |
| Multiple windows | Explicitly unfinished in baseline README | Application runtime, per-window drivers/resources | 1 skeleton, 4 complete; beyond parity |
| App-declared menus | Explicitly unfinished in baseline README | Shared Command registry and platform menus | 2/4; beyond parity |
| Tabs and split panes | Compose from primitives, not a built-in baseline API promise | Tabs and Split_pane | 4; no full docking requirement |
| Test renderer, semantic automation, screenshots, clock | testing.ts; native index.d.ts; automation | Gpuio_test and deterministic reference-app tests | From 1, complete 7 |
| Runtime errors/frame metrics | README debug overlays and status | Diagnostics, error surface and trace hooks | From 1, complete 7 |
| Standalone distribution | README shipping recipes | Dune/Rust build integration and app packaging | 1 build recipe, 7 clean-machine install |
| Embedded updates | README updater and native [updater.rs](<http://updater.rs>) | Optional platform integration | Optional platform integration; not a component-parity release blocker |
| JS remount/hot reload | README Bun development flow | Ordinary build/restart workflow | Explicitly excluded from parity and v1 requirements |
| Canvas | Marked planned in baseline; GPUI exposes native drawing | Typed retained scene and interaction API | Required expanded v1: [OCH-24](<https://linear.app/ochat/issue/OCH-24/implement-typed-retained-canvas-drawing-and-native-interaction>) |
| Windows/web/WASM | Baseline has these paths | No Windows or browser target | Excluded by project scope |

The ledger records GPUIX's documented nested-scroll limitation; it is not proof that all upstream GPUI nested scrolling is unsupported. GPUIO must test wheel routing for transcript, composer, popup and horizontal code scrolling on its pinned GPUI. Avoid copying the limitation or asserting it is solved without that evidence.

The initial prototype covers only a fraction of this table. A v1 claim requires each committed row to have a semantic/native test, documentation and platform status. Shipping a rich control set with missing keyboard, editing or accessibility behavior is not equivalent parity.

---

## notes/api-design/gpuix-inventory.json

```json
{
  "baseline": "GPUIX 18e695ed0ee8121a7793413ca795e08eda2a13df",
  "review_date": "2026-09-10",
  "status": "Source inventory, not implemented GPUIO coverage",
  "style_fields": [
    "display",
    "visibility",
    "flexDirection",
    "flexWrap",
    "flexGrow",
    "flexShrink",
    "flexBasis",
    "alignItems",
    "alignSelf",
    "alignContent",
    "justifyContent",
    "gap",
    "rowGap",
    "columnGap",
    "gridTemplateColumns",
    "gridTemplateRows",
    "gridColumnMin",
    "gridRowMin",
    "width",
    "height",
    "minWidth",
    "minHeight",
    "maxWidth",
    "maxHeight",
    "padding",
    "paddingTop",
    "paddingRight",
    "paddingBottom",
    "paddingLeft",
    "margin",
    "marginTop",
    "marginRight",
    "marginBottom",
    "marginLeft",
    "position",
    "top",
    "right",
    "bottom",
    "left",
    "background",
    "backgroundColor",
    "color",
    "opacity",
    "borderWidth",
    "borderTopWidth",
    "borderRightWidth",
    "borderBottomWidth",
    "borderLeftWidth",
    "borderColor",
    "borderRadius",
    "borderTopLeftRadius",
    "borderTopRightRadius",
    "borderBottomLeftRadius",
    "borderBottomRightRadius",
    "boxShadow",
    "fontSize",
    "fontFamily",
    "fontWeight",
    "textAlign",
    "lineHeight",
    "whiteSpace",
    "textOverflow",
    "lineClamp",
    "textDecoration",
    "overflow",
    "overflowX",
    "overflowY",
    "cursor",
    "pointerEvents",
    "userSelect",
    "selectionColor",
    "hover",
    "active"
  ],
  "events": [
    "`onClick`",
    "`onAuxClick`",
    "`onMouseDown`",
    "`onMouseUp`",
    "`onMouseEnter`",
    "`onMouseLeave`",
    "`onMouseMove`",
    "`onMouseDownOutside`",
    "`onKeyDown`",
    "`onKeyUp`",
    "`onFocus`",
    "`onBlur`",
    "`onScroll`",
    "`onFileDrop`",
    "`onChange`",
    "`onSubmit`",
    "`onToggleFile`",
    "`onShowMore`",
    "`onLineClick`",
    "`onLinkClick`"
  ],
  "source_files": [
    "README.md",
    "packages/react/src/types/host.ts",
    "packages/react/src/index.ts",
    "packages/native/index.d.ts"
  ],
  "explicitly_planned_not_implemented_in_baseline": [
    "canvas",
    "multiple windows",
    "app-declared menus",
    "React Refresh"
  ]
}
```

## Required additions beyond the pinned GPUIX parity baseline

| Capability | Owning issue | Milestone |
| -- | -- | -- |
| Implement the statically linked native component extension SDK | [OCH-23](<https://linear.app/ochat/issue/OCH-23/implement-the-statically-linked-native-component-extension-sdk>) | 05 |
| Implement typed retained canvas drawing and native interaction | [OCH-24](<https://linear.app/ochat/issue/OCH-24/implement-typed-retained-canvas-drawing-and-native-interaction>) | 05 |
| Extend native animations with springs, sequences and synchronized repetition | [OCH-25](<https://linear.app/ochat/issue/OCH-25/extend-native-animations-with-springs-sequences-and-synchronized>) | 05 |
| Implement native declarative container-size breakpoint rules | [OCH-26](<https://linear.app/ochat/issue/OCH-26/implement-native-declarative-container-size-breakpoint-rules>) | 05 |
| Implement deep links and desktop application/document integration | [OCH-27](<https://linear.app/ochat/issue/OCH-27/implement-deep-links-and-desktop-applicationdocument-integration>) | 06 |
| Implement capability-aware OS notifications and action routing | [OCH-28](<https://linear.app/ochat/issue/OCH-28/implement-capability-aware-os-notifications-and-action-routing>) | 06 |
| Validate expanded v1 with a graphics application and independently packaged native component | [OCH-29](<https://linear.app/ochat/issue/OCH-29/validate-expanded-v1-with-a-graphics-application-and-independently>) | 06 |

The pinned GPUIX inventory below/above remains historical evidence of that library, not GPUIO's full current scope. Optional macOS native tabs/Dock, Wayland layer-shell and surface support are tracked in <issue id="0f9f4312-8b52-4a88-a6d1-7457fe22677a" href="https://linear.app/ochat/issue/OCH-30/optional-add-macos-native-window-tabs-and-dock-integration">OCH-30</issue>–<issue id="a9c40d72-c648-4536-a74a-b4aa897c7eef" href="https://linear.app/ochat/issue/OCH-32/optional-investigate-and-expose-supported-native-pixel-buffer-surfaces">OCH-32</issue>. See the [expanded scope](<https://linear.app/ochat/document/gpuio-expanded-v1-scope-drawing-extensions-motion-and-desktop-services-46c91a3056cd>) for complete contracts and platform limitations.

