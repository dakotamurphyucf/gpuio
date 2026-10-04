# Window presentation and composed controls

OCH-41 contract, extending the current custom-chrome work. Implementation and
validation evidence remain separate from the complete frame/release acceptance.

Every Window.Snapshot carries Presentation: actual Decorations (Server or Client
with four tiled edges), Controls (fullscreen/maximize/minimize/window menu), and
resizable policy. The host reads native GPUI state, rather than assuming a chrome
request was honored. Minimize and maximize capabilities also honor the window's
minimizable/resizable policy. Fullscreen follows the backend's capability flag.
Snapshots remain asynchronous observations, not promises of OS transition completion.

Bounds/activation observations retain their existing behavior. Rendering detects
presentation changes independently, so a compositor capability/decorations update
without a size change still produces a coalesced WindowChanged event. There is no
polling timer or synchronous OCaml call. Native minimize/zoom/fullscreen commands
recheck live capabilities and return Unsupported when unavailable, including when
an ordinary button click arrived from a formerly supported state.

View.window_controls composes ordinary accessible buttons. It is empty on macOS
or with server decorations, avoiding duplicate native controls. Linux client
decorations show supported Minimize and Maximize/Restore, and always Close. The
application supplies callbacks, including its existing request_close decision.
Normal keyed reconciliation, focus, accessibility, theme styling and callback
generation rules apply; no new native button/event/focus ownership is introduced.

The gallery uses observed snapshots for its custom-chrome controls. The native
[window-owned frame](window-frame.md) supplies client-frame geometry, shadows and
tiling-aware resize hit testing. Presentation observations do not themselves
prove compositor or desktop acceptance.

The unpublished epoch-3 Window.Snapshot appends Presentation after Document.
Existing enum tags remain fixed; producer/consumer must be built together.
Independent byte fixtures cover server/client state and changed capabilities.
