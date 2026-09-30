# Disabled native subtrees

Status: implemented for OCH-41 Settings, with scoped local macOS native evidence
below. Complete Settings parity, VoiceOver, OS IME and release qualification
remain separate acceptance work.

`Style.Disabled true` is a base-only, inherited interaction policy. It preserves
layout, paint, accessible names, roles and values while excluding descendants
from user activation, editing and keyboard focus. Accessibility exposes disabled
state and no mutating actions. `Disabled false` clears the local declaration;
it cannot re-enable a descendant of a disabled ancestor. Independently disabled
controls remain disabled when their ancestor is re-enabled.

This differs from `Inert`, which intentionally hides its accessibility subtree
and remains appropriate for retained inactive pages. Neither policy cancels
application tasks or owns saved application data. Disabled adds no implicit
opacity; the Settings presentation supplies its existing subdued appearance.

Native controls and the extension SDK keep their state ownership. Current-tree
guards protect input and retained accessibility objects, including after an
enabled control is disabled. Disabling retires focus and transient interaction
such as popups/capture; restoring availability does not steal focus or replace
editor identity. Application-originated commands retain each widget's documented
contract; focus/submit cannot enter disabled content.

The wire addition is Boolean style field 69, negotiated by bit 54
(`CAP_DISABLED_SUBTREES`). The paired required mask is `36028797018963967`.
Independent Hello bytes are `0001fc0000000000004000` for this bit alone and
`0001fcffffffffffff7f00` for the complete current mask. Older hosts reject the
required capability rather than silently ignoring this policy.
Hover/focus/pressed declarations
reject it atomically, as they do Inert. Rendering visibility stays separate from
interaction eligibility. Native accessibility inheritance is applied to the
completed frame's output, without changing retained source nodes; re-enabling
must restore the original actions and preserve individually disabled descendants.

Native overlays and palettes retire their transient surface and focus scope
while unavailable. Application-controlled overlay declarations remain owned by
the application; subsequent availability follows their existing autofocus
contract. This does not forcibly erase controlled overlay state. Platform menu
bars remain present but disabled, including nested entries. A retained OS action
checks the current menu policy even if its command resolves outside that menu.

## Local validation — 2026-09-30

macOS 14.5 arm64, stock repository OCaml/Rust/GPUI pins. This evidence covers
scoped behavior, not a whole-application performance or Linux GUI qualification.

- Independent OCaml/Rust fixtures verify field 69 Boolean bytes, negotiation,
  truncation, invalid Boolean and trailing-byte rejection. Core state styles and
  native atomic admission reject state-dependent disability; a failed batch
  preserves its preceding revision, text and interaction policy.
- Native navigation checks disabled ancestry with a local false descendant,
  stable geometry/editor identity, rejected keyboard/pointer/AX activation,
  `AXEnabled=false`, blocked AppKit text input and re-enabling. Actual GPU pixels
  verify no added opacity. Existing Inert accessibility hiding still passes.
- A running textarea drag-selection timer stops after either Inert or Disabled
  is applied, without waiting for mouse-up; selection stays intact and rendering
  becomes idle. Pointer capture cancels with the Disabled reason.
- A live select popup closes on ancestor disable, releases focus, rejects new
  activation and preserves its owner when restored. A custom native extension
  rejects retained event-sink callbacks and AX presses, preserves its instance,
  and resumes input without remounting or stealing focus.
- macOS native menu tests reproduce a menu remaining enabled before the routing
  repair. The repaired snapshot keeps it present but disabled; retained actions
  cannot invoke a command outside the disabled menu. Re-enabling restores real
  NSMenu activation. Full controls also cover palette close-before-command Copy,
  nested focus, overlays, tooltip timers and ordinary enabled input.
- All 14 pinned GPUI accessibility unit tests pass, including disabled ordinary
  and synthetic descendants, original values/actions, re-enabling and independent
  siblings. The scoped prepaint API fixes a real public-gallery failure: metadata
  attached only to a type-erased wrapper did not reach the actual native nodes.
  See [the GPUI patch contract](gpui-core-adaptation.md#disabled-accessibility-scopes).

Commands (native executables run sequentially under a 180-second process-group
watchdog and close/reap normally):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-protocol
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --lib
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --test session
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests --test native_navigation --test native_pointer --test native_extension --test native_controls --test native_menus --no-run
# Run the Cargo-reported test executables with a bounded watchdog.
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native --features native-image-tests --all-targets -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
```

The GPUI unit suite uses an isolated copy of the vendored crate under the pinned
upstream `crates/gpui` directory shape, with the two upstream font assets and
`test-support,gpui_platform/runtime_shaders`. Its own test-only manifest/lockfile
does not change the production dependency pins or shared Cargo sources. The
recorded GPUI archive and patch reconstruct byte-for-byte to `vendor/gpui`.

The native unit checkpoint passed 422 tests with two existing ignored tests;
the protocol suite, six native session tests and strict native Clippy passed.
Direct native text-client calls in these fixtures are distinct from actual OS
input-method candidate-window acceptance. No VoiceOver speech claim is made.

The repository Settings walkthrough and a freshly staged installed-library
consumer pass public `AXEnabled`/identity checks, OS keyboard activation,
disabled dirty-value reset exclusions, and attempted AXPress through an object
retained before disabling. They also retain the existing split resizing,
Unicode editing, search/page recovery, native Save/Eio readback, 48-group
focus/eviction and independent-window checks. The consumer is
`/private/tmp/gpuio-disabled-subtrees-20260930-1`; it is evidence only, never a
build dependency. See [Settings evidence](settings-composition.md#disabled-field-discoverability-follow-up--2026-09-30).
