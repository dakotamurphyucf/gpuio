# Toast stack placement — OCH-41

Implemented with local Core/codec/admission/native Host evidence; see the
[verification checkpoint](../evidence/toast-placement-och41.md). Physical/release
qualification remains open. Follows the pinned notification review. Existing
four-corner stacks, their timeout clocks and terminal dismissal stay supported.

`Toast.Placement.Anchor` has eight anchors: the four corners and top, bottom,
left and right center. `Toast.Placement.create ~anchor ?top ?right ?bottom ?left ()`
returns a checked abstract placement. Insets default to 16 logical pixels and
must be finite in 0..16384. They describe the usable rectangle inside the window,
not padding on toast content. Center alignment uses that usable rectangle.

`Toast.Stack.create` accepts optional `~placement`; supplying both `~corner` and
`~placement` is an error. With neither, preserve the existing bottom-right/16px
contract. Placement changes preserve stack and toast identities, child editors,
elapsed active timeout, callbacks and focus. They do not restart or reopen a toast.

Use an additive optional native placement operation. `None` restores the existing
corner and default insets. Validate both decoding and native admission; reject a
non-stack target atomically. Core compares placement separately from the original
stack metadata, so no-op frames emit nothing and clearing an override is explicit.

The native viewport frame measures placement each render. Clamp each opposing
inset pair proportionally when it exceeds the available dimension, leaving zero
usable extent rather than negative geometry. Clamp stack width/maximum height to
the usable rectangle and clip impossible content. Width/height remain logical
pixels and resizing requires no OCaml layout callback or permanent timer.

Verify independent paired operation bytes, malformed bounds/anchors, atomic
admission, public reconciliation/reset/no-op and all eight native anchors with
asymmetric insets and tiny windows. Native retained-editor/focus and idle checks
must cover placement updates. Gallery controls demonstrate anchors/insets. Physical
macOS/VoiceOver/GPU qualification remains distinct from TestPlatform evidence.

Layered expansion and enter/exit/reflow are separate required work under the
notification review; this placement contract does not claim those implemented.
