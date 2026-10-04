# Runtime and construction helpers — OCH-41

Reviewed 2026-10-04 against GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. Eight original Base/Component
runtime-helper sources, including Component Root, are retained in
[the source manifest](sources/manifest.json). The Component traits/global/index
modules mostly re-export Base; they are not three additional widget implementations.

| Pinned behavior | GPUIO ownership and public mapping |
| --- | --- |
| `async_util` native/WASM unbounded channel aliases | OCaml application work uses `Gpuio_eio.Scope`, Eio capabilities and bounded `Inbox`/stream delivery. Rust workers remain native. The project's bounded bridge/cancellation contracts take precedence over copying an unbounded channel alias. There is no Async or browser runtime requirement. [Runtime contract](../design/runtime.md). |
| `Selectable`, secondary selection, `Disableable`, `Collapsible` | Controlled selection/disabled/open values on the relevant widget APIs, plus explicit style states. Secondary selection has meaning only on widgets that expose it; a trait's default no-op is not a promise of a new universal state. Component-specific configuration/semantics remain authoritative. |
| `FocusableExt` focus-ring state | Native focus ownership and each control's focused styles/appearance. There is no separate shared mutable focus-ring object. A style change does not move keyboard focus or bypass accessibility semantics. |
| `GlobalState` initialization, menus, popup registration, selection suppression/order | Rust owns the per-application Base infrastructure used by native adapters. OCaml app menus/commands and overlay state use typed descriptions. Selection owners suppress competing pointer gestures and allocate logical selection order; weak popup tokens avoid keeping removed popups alive. Callers do not mutate this native global from OCaml. |
| `IndexPath` section/row/column builders, formatting, row equality | List/tree/table models use explicit record keys and column identities; grouped choice/picker APIs own their section structure. Positional coordinates can be ordinary OCaml records/values when needed. Upstream index formatting is not a stable identity or a serialization contract. |
| Root view storage and global read/update closures | `App.run`, generation-checked `App.Window.t`, one Bonsai driver per window and a Rust retained `View` owner. Application data can be shared explicitly; no arbitrary synchronous OCaml root-update closure runs inside GPUI. |
| Root stacked dialogs/sheet, previous-focus restoration and notifications | Declarative Dialog/Sheet/Toast compositions, native modal focus/selection scopes, keyed lifetime and focus-return policy. See [overlay](overlay-review.md), [notification](notification-review.md) and [window](window-review.md) mappings. Render builders become descriptions; native focus restoration remains native. |
| Root tooltip and fallback-menu overlays | Existing managed native tooltip/menu hosts with bounded deadlines, active routes and native placement. No second provider, popup manager or async scheduler is introduced by this mapping. |
| Root Tab/Shift-Tab, Copy and active selection scope | Native focus traversal/modal gates and shared selection ownership. Window-wide selection helpers now have an explicit bounded asynchronous contract. GPUIO preserves selected whitespace rather than copying Root's trailing `trim()` shortcut. |
| Root base font/background, Linux border/shadow and child rendering | Per-window resolved styles, `Window.Config`/client-frame policy and retained child composition. System decorations remain backend-owned. [Window contract](../design/windows.md), [client frames](../design/window-frame.md). |

## Evidence and continuation

The gallery's Runtime, Overlays, Settings and Navigation pages exercise these
owners through documented APIs. `test/runtime/scope_test.ml` verifies selective
cancellation, suppressed queued completions, bounded admission and lifetime
counts; driver/reconciler tests cover keyed replacement and stale routing. The
full OCaml suite passed at the [current helper checkpoint](../evidence/window-selection-och41.md).
Two pinned Base global-state tests (idempotent initialization/selection suppression
and dropped popup-token release) and three IndexPath tests also pass in the
repository environment using the standalone locked command/overrides from
[the Base test instructions](../evidence/document-accessibility-och17.md), with
`--lib global_state` and `--lib index_path` respectively.
Existing [runtime measurements](../evidence/runtime-och9.md) are dated samples,
not current release performance budgets. The accepted configurable Bonsai clock
polling remains unchanged; no native idle-redraw guarantee follows from a clock
tick count alone.

The geometry and focus operations invoked by Root remain subject to their
component/platform evidence. Its macOS accessibility hit-test installation is
tracked explicitly in [the diagnostics review](diagnostics-review.md); importing
Base does not prove the host calls it. Whole-app teardown, physical keyboard/IME,
current resource/performance workloads and distribution remain release gates.
