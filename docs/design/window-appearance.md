# Native window appearance observations

Implemented locally from the OCH-41 theme audit, 2026-10-04; see
[validation and remaining platform checks](../evidence/window-appearance-och41.md).
`App.Window.set_theme` updates application color tokens; native appearance is
reported separately and setting color tokens does not change OS decorations.

Pinned Component `Theme.sync_system_appearance` reads the native window appearance
and maps it to its light/dark theme. GPUIO needs equivalent information so an OCaml
application can choose its own theme without polling or synchronous native-to-OCaml
callbacks. This is application functionality, separate from theme-file watching or
development hot reload.

## Public shape

`Window.Appearance.t` has the four native alternatives `Light`,
`Vibrant_light`, `Dark`, `Vibrant_dark`, plus `is_dark : t -> bool`.
`Window.Snapshot.t` includes `appearance : Appearance.t`. The existing
`App.Window.snapshot`, `on_change` and `command Observe` then carry it without a
second subscription API or a new pending-request lane.

Appearance means the resolved appearance of that native window at observation.
It is not a measurement of the drawn background, an application color palette or
an assertion that the user has never forced an OS appearance elsewhere. Preserve
the native distinction between vibrant and ordinary appearances; applications
needing two palettes can use `is_dark`. A platform may only produce ordinary light
and dark values. Do not infer an unreported high-contrast or accessibility setting.

The application chooses `System`, `Light` or `Dark` policy in its model. System
maps observed native appearance to its chosen palette and calls `set_theme` only
when that choice changes. Explicit application light/dark choices keep their
colors when native appearance changes. This preserves per-window theme ownership.
This addition does not force an application-wide OS appearance: GPUI's override
is platform-dependent and would require a separate capability contract.

## Native ownership and transport

Use the pinned GPUI `Window.observe_window_appearance` notification, keeping its
subscription with the native window owner. Initial observations must include the
actual current appearance even if no change notification arrives. The callback
uses weak native ownership, publishes through existing coalesced window metadata
and wakes ordinary asynchronous delivery. It must not create a recurring timer,
retain a closed window or synchronously call OCaml.

Window/node generations, close cleanup and wrong-window response handling remain
under the existing window contract. A replacement native window must not receive
an old callback. Unchanged metadata should retain normal coalescing/no-op behavior;
an appearance change is not a new editor, document or Bonsai component mount.

The paired unpublished epoch-3 snapshot codecs append an appearance field with
tags 0..3 in the order above. Both sides must be built from the same sources;
this is not an optional payload or compatibility with an older native binary.
Independent fixtures cover command replies and lifecycle observations, with
malformed/truncated payload rejection on the OCaml receiving side.

## Required validation and example

- Independent Rust/OCaml fixtures for every native appearance and invalid tags.
- Native TestPlatform initial state and appearance changes, including a change
  before first paint, multiple windows and closed/stale owner cancellation.
- A public gallery System/Light/Dark control. Following a native change must
  preserve child editor identity, draft, focus and selection; explicit modes must
  remain selected. An appearance event must not overwrite other registered
  window-observation behavior.
- Idle/change coalescing and no extra periodic wake source, plus normal full
  protocol/native/OCaml/gallery/lint checks.
- Separately record physical macOS System Appearance switching and Linux backend
  reporting. Simulated changes prove routing, not OS preference integration.

The gallery keeps its explicit Dark startup palette and offers **Follow system**
beside the Light/Dark toggle. The selected system policy maps both vibrant and
ordinary native variants to the application's two palettes. Clicking Light/Dark
selects an explicit palette again. Each window owns its preference independently.
The same window observation handler feeds custom chrome and theme choice; the
appearance feature does not replace that handler. Palette changes use the existing
Bonsai graph and editor controller identities.

The broader style/theme source audit remains open, including nested theme schema,
registry, color/motion utilities and component-specific defaults. This focused
addition must not be described as full theme or release acceptance.

The [physical macOS walkthrough](../evidence/window-appearance-och41.md#physical-macos-appearance--2026-10-05) now passes on macOS 14.5: real OS transitions, independently rendered window palettes, retained selection/native editing, and verified restoration of the original preferences. Automatic scheduling, high contrast and Linux desktop reporting remain separate.
