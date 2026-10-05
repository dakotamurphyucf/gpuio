# Window and chrome source review — OCH-41

Reviewed 2026-10-04 against GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. This is a feature-level mapping,
not acceptance of the window family. The three unmodified title-bar, window-border
and window-extension sources are retained in [the manifest](sources/manifest.json).
The added `window_ext.rs` snapshot comes from the existing archive after verifying
its pinned SHA-256; no dependency revision changed.

| Source behavior | GPUIO mapping and remaining work |
| --- | --- |
| Native windows, title, content size, activation, zoom/fullscreen | `Window.Config`, `Window.Command`, `App.open_window_config` and generation-checked `App.Window` requests. Commands observe state at execution; a reply does not acknowledge completion of an asynchronous OS transition. |
| Standard title-bar controls | `Chrome.Standard` uses platform-owned decorations. Existing native close/quit decision routing and `request_close` preserve application-owned unsaved-work policy. Standard OS controls do not establish custom-chrome parity. |
| Programmatic minimize / custom minimize button | `Window.Command.Minimize` now uses the existing request lane, invoking GPUI on the native thread; [local wire/build evidence](../evidence/window-minimize-och41.md) is recorded. It must preserve the window, editing sessions and Eio scope. The response is an ordinary window snapshot; no minimized-state observation exists in the current wire snapshot. Actual macOS minimization/restoration now passes three cycles in each of standard/custom modes with retained editor state; [physical evidence](../evidence/window-lifecycle-och41.md) keeps in-flight task qualification separate. |
| Custom `TitleBar`, child content and background | `Chrome.Custom`, `View.title_bar` and `View.with_window_region` now configure transparent/app-owned chrome, native traffic lights at (9,9), styled content and observed-fullscreen reservation. The public gallery has `--custom-chrome`. The [physical macOS walkthrough](../evidence/window-lifecycle-och41.md) now covers movement, fullscreen/title-bar layout, AppKit resize and double-click zoom; see [the contract](../design/window-regions.md). |
| Drag to move and title-bar double click | Implemented as native admitted regions with current-node/config identity checks, cancellation and child-control exclusion. TestPlatform covers request routing, button/editor input and state retirement; actual macOS movement/default double-click zoom now passes in the [public gallery](../evidence/window-lifecycle-och41.md). Other preference modes and child policies retain separately scoped coverage. |
| Close, zoom/restore and minimize controls in client decorations | Existing commands/close decisions cover some actions. The custom gallery uses ordinary buttons and current-window actions. `View.window_controls` now composes supported client controls from actual `Snapshot.presentation`, preserves ordinary button semantics and requests the application close decision. Native command guards recheck current platform support. Physical keyboard/AX qualification remains; see [presentation contract](../design/window-presentation.md). The pinned min/max/close control-area tags are Windows-only; pinned macOS/X11/Wayland hit-test callbacks ignore those tags. The close button must request the application decision, not force removal. |
| Client `WindowBorder`: edge/corner resize, tiling-aware padding, cursor and shadow | Automatic window-owned framing is implemented for Linux `Chrome.Custom` when actual decorations are Client: immutable validated geometry, stable platform inset, per-edge tiling/fullscreen layout, themed border/shadow, native resize bands and content-aware overlay fitting. Explicit resize regions remain available. macOS/server frames stay native. See [frame contract](../design/window-frame.md) and [local evidence](../evidence/window-frame-och41.md); real compositor qualification remains deferred to OCH-47. |
| `WindowExt` dialog/sheet/alert operations | Map to application-owned open state and existing Dialog/Sheet/Alert compositions and overlay lifecycle. Rust builder closures and upstream `Root` storage do not need one-to-one FFI wrappers. See [overlay review](overlay-review.md). |
| `WindowExt` keyed notifications | Map to keyed application Toast state and native rendering/lifetime. These are in-window notifications, separate from OS notifications. See [notification review](notification-review.md). |
| `WindowExt` focused-input lookup | `App.Window.focused_input` now returns metadata for the current eligible native input, with exact window/node identity and matching helpers for existing controller snapshots. Kinds include editors and composite native text fields. Read-only inputs remain discoverable; hidden/disabled/removed/modal-blocked owners and color non-text controls are excluded. No text or Rust entity crosses this query. Option presence maps `has_focused_input`. See [contract](../design/window-input-query.md) and [local plus physical macOS evidence](../evidence/window-input-query-och41.md). The actual public shortcut now passes ordinary/masked/read-only owners, native editing, independent windows and remount identity; other kinds/exclusion rules retain their separately scoped coverage. |
| Deprecated window selection read/clear/end helpers | `App.Window.has_text_selection`, `selected_text`, `clear_text_selection` and `end_text_selection` map the registered read-only selection layer. Reads respect the active native scope and renderer copy normalization, with a checked UTF-8 byte budget and explicit oversize failure. Clear removes registered selections; end stops dragging while preserving the range. Editable input values and clipboard access are excluded. See [contract](../design/window-selection.md) and [local checks/reconstruction and physical macOS walkthrough](../evidence/window-selection-och41.md). Exact cross-node Unicode read, end-drag retention, clear, window isolation and native remount now pass through the public gallery. Other renderer/modal/editor-exclusion cases keep their separately scoped evidence. |

## Evidence and continuation

Inspect `lib/core/window.mli`, `lib/protocol/window_wire.ml`, `lib/eio/app.mli`,
`rust/native/src/window_host.rs` and native window construction in `host.rs`.
The Runtime gallery shows observations/file selection/minimize, and the opt-in
`--custom-chrome` shell now demonstrates title-bar/controls and automatic client
framing on supporting Linux backends. Historical window/close/quit evidence
is in [the workspace record](../evidence/agent-workspace-m4.md), with macOS
accessibility limitations in [the window record](../evidence/window-accessibility-och17.md).

Keep this family open until the remaining helper mapping work, physical gallery
and meaningful platform evidence exist. Map geometry/runtime
helpers independently; source presence and a generic root-module entry cannot
close their reviews. No new exclusion or post-v1 deferral is introduced here.
