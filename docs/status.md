# Implementation status

Updated 2026-09-13. Milestone 01: reproducible foundation, complete. Milestone 02: native interaction, in progress.

Repository: `dakotamurphyucf/gpuio`, public, Apache-2.0, default branch `main`.
These settings were selected by the owner on 2026-09-11.

Platform priority updated by the owner on 2026-09-11: macOS is the primary
functional acceptance platform during implementation. Linux builds/unit tests
remain required, but graphical checks are informational and full Linux GUI
validation is deferred to OCH-17. Linux remains an intended platform. Earlier
design documents requiring native GUI acceptance on both OSes before advancing
are superseded by this priority; native GUI coverage must still be reported honestly.

- OCH-18 complete: remote scaffold, standards/design import and fresh-clone checks.
- OCH-19 complete: pinned OCaml/Rust closure, reconstructed
  native Bonsai sources and patches, codec/lifecycle checks.
- OCH-20 complete: isolated bootstrap and contributor tools.
- OCH-21 complete: source-built Dune/Cargo smoke app and
  two-window native identity/lifetime scenario.
- OCH-22 complete: required builds/tests, macOS native checks, informational Linux
  graphical checks, retained evidence and protected main branch.
- OCH-6 setup gate complete, merged in PR #1 at `81f6b581c784d448a8948d4cb55e73af9db4b86c`.
- OCH-7 complete, merged in PR #2 at `e6471b4ec88e6847f950da30576b6d2a6639d930`.
  CI run34650637422 passed on both OSes, including production 50-revision/
  two-window/rollback/panic smoke on macOS, X11 and Wayland.
- OCH-8 complete, merged in PR #3 at `8f7fd9f357a0b8df3e9dfe31c2a7217925c8846d`: typed views/styles/themes, keyed reconciliation, pure Bonsai
  adapter, native button/selection behavior and GPUIX style mapping. Local macOS
  tests and both required CI jobs passed in run 34654290650. X11 passed all GUI
  checks; Wayland passed the typed bridge but failed the new hover-reset test.
  That informational limitation remains tracked in OCH-17.
- OCH-9 public Bonsai/Eio runtime merged in PR #4 at
  `02558d8d393c49e5e159812394dd9061820c39fc`. Both required CI jobs passed in
  run 34740866262, including all macOS runtime/measurement scenarios. Linux GUI
  exposed a default quit-policy difference, fixed in PR #5 at
  `88cc9287db49cd27c0b78a6f19eea17fc4c1069e`. Final run 34741216419 passed both
  required jobs and all OCH-9 scenarios on macOS, X11 and Wayland. X11 passed
  the full GUI suite; the existing Wayland hover-reset issue remains under OCH-17. See [runtime](design/runtime.md) and [measurements](evidence/runtime-och9.md).

## Local evidence

macOS arm64, stock OCaml 5.3.0, Dune 3.24.2, Rust 1.97.1. The separate
`.opam-root/gpuio` was created from the pinned opam repository; the existing Ochat
switch and default toolchain selections were not modified.

- Core/PPX expect tests and native Bonsai lifecycle tests pass, including
  optimized/unoptimized graphs, unchanged views, keyed retention, cleanup and a
  dedicated OCaml domain.
- The OCaml and Rust codec checks independently construct, encode and decode the
  same 96-byte fixture with full byte consumption.
- The actual GPUI window self-test passes: 50 checked commits, 1525 command bytes,
  native input-handler probes, stale event rejection, Rust panic containment,
  balanced 23/23 row activation/deactivation, Eio cancellation and clean shutdown.
- Two actual windows pass distinct identity, independent editor state, stale
  window rejection and continued use of the surviving window after closing the
  first. Both close and return through the FFI.
- First-party Rust passes Clippy with warnings denied. GPUI's transitive `block`
  0.1.6 reports a future-compatibility notice; it does not fail the pinned build.

`docs/evidence/macos-arm64-packages.txt` is the actual isolated package inventory.
Historical research documentation is preserved under `docs/design` and is not
an assertion of current production API functionality.

## Hosted evidence and remaining platform validation

PR run [34646959232](https://github.com/dakotamurphyucf/gpuio/actions/runs/34646959232)
passed on macOS ARM64 and Ubuntu 24.04 x86-64. The informational Linux GUI report
also records X11 and Wayland success: both asserted the intended backend and
passed the 50-commit native/lifecycle/input-handler example and two-window
identity/cleanup scenario. X11 used Xvfb/Openbox; Wayland used nested Weston;
Mesa software Vulkan supplied rendering. This is actual backend window coverage,
distinct from the earlier accidental headless X11 attempt. Both pinned language servers have passed hover and
go-to-definition checks; evidence is in `docs/evidence/*-lsp-navigation.json`.
Main requires PRs and both `foundation (macos-15)` and `foundation (ubuntu-24.04)`
checks with an up-to-date branch. Force pushes and branch deletion are disabled.
Linux GUI outcomes remain informational and do not alter this development gate.
No full OS IME automation, accessibility or production multi-window Bonsai API is
claimed by the bootstrap smoke tests. Those remain in their owning v1 tickets.

## Typed API validation (OCH-8)

The pure API tests cover callback-only refresh, keyed reorder/replacement, invalid
plans, theme changes, style composition/reset and bounded incremental output.
OCaml and Rust independently agree on every expanded style tag in `style-v1.hex`.
Native tests validate malformed styles, rollback and nested memory accounting.
The actual macOS window test passes grid bounds, hover/pressed/focus, Enter/Space,
Tab/Shift-Tab, pointer policy, Unicode select/copy, replacement and inherited reset.
The public OCaml example passes 20 acknowledged native commits and theme changes.
The [typed API contract](design/typed-ui.md) records all GPUIX style mappings and
functional limits. Linux graphical execution remains informational under OCH-17.

## Milestone 02

OCH-10 implements native input/composer ownership, stable Bonsai/Eio controllers,
revisioned commands, native composition and grapheme editing, undo/redo selection,
auto-grow and basic accessibility. [PR #6](https://github.com/dakotamurphyucf/gpuio/pull/6)
and [its evidence report](evidence/native-editor-och10.md) record implementation
and platform validation. Hosted run 34745383026 passed Linux build/tests/lint and
macOS editor/input/accessibility checks. X11 passed the complete GUI suite;
Wayland passed public editor commands but its clipboard-based native test failed
before insertion, tracked in OCH-17. These checks do not claim physical IME
candidate-panel or complete screen-reader coverage.

OCH-11 is in progress: controlled checkboxes/switches and disabled buttons merged
in PR #7 (`0ef2c7dc5b71235c090d4dc6373f505db69624e5`); CI 34747606484 passed both
required jobs and control windows on macOS, X11 and Wayland. The existing Wayland
editor clipboard limitation remains under OCH-17. Radio groups in PR #8 pass
both required jobs in CI34748629291, including actual control windows on macOS,
X11 and Wayland. PR #8 merged as `5448842ffd9bff9e249071698a294a3afc3ffb42`.
Select PR #9 merged as `8a9167cf227a290f967452c051f0b4c7dde19461` after both required
jobs in CI34750240273 passed. Its adapter adds native popup navigation/cancellation, current-frame
positioning and virtualized options. Local macOS window/accessibility checks,
4096-option navigation, OCaml/Rust tests, full build/format and Clippy passed.
Choice appearance adds theme-aware popup/option/empty styles, configurable uniform
row geometry and localized empty text while retaining native focus/open state.
Local tests validate these changes; general overlay integration remains OCH-11 work. [Native controls](design/native-controls.md) records these families
and the remaining ticket scope. OCH-12 declarative animations follows.
Combobox is implemented on the local OCH-11 branch with native editor ownership,
query filtering, exact selection snapshots, the editable accessibility role and
shared popup appearance/virtualization. Local native control tests pass including
macOS marked/committed text. The public controller smoke passes conditional
replacement, stale revisions, undo and unmount; existing two-window editor commands
also pass after sharing the controller implementation. OCaml/Rust tests and Clippy
pass locally. Full build/format validation is recorded with the local change.
Per owner instruction, remaining OCH-11 work stays local until the complete ticket
is ready for a consolidated CI pass. No Combobox hosted acceptance is claimed.
The broader component catalog is planned in OCH-33–45; vendoring GPUI Base does
not expose all of those widgets through the OCaml API.

Focus scopes are also implemented locally for OCH-11: native Tab trapping, nested
entry/restoration, hidden/disabled traversal, empty-root fallback, command and
accessibility gating, and bounded cleanup pass actual macOS control-window tests.
The other remaining OCH-11 families are still in progress.
No hosted acceptance is claimed for this local scope implementation.

Dialog/Popover surfaces are implemented locally with application-controlled
content lifetime, typed dismissal, native stacking/placement and accessibility
semantics. Local macOS tests pass nested dialogs, restoration, choice-popup
interaction beyond panel bounds, marked-text Escape and moving anchors. The
public Bonsai/Eio overlay example passes native mount, editor commands, modal
focus denial, stale unmount and close. OCaml/Rust tests, independent fixtures,
full build/format and Clippy pass locally. No hosted acceptance is claimed;
tooltips/menus/commands and the rest of OCH-11 remain in progress.

Anchored placement is implemented locally: popovers accept preferred side,
start/center/end alignment and signed offset, with current-frame edge flipping
and viewport clamping. Local native checks retain focus while changing placement
and moving the anchor; positioning/validation unit tests and independent protocol
fixtures pass. The extension appends a new operation without changing earlier
overlay records.

Tooltips are implemented locally with managed or application-controlled visibility,
retained arbitrary content, delayed hover, shared grace timing and keyboard
opening/dismissal. Hidden content preserves native editor identity while denying
focus and deactivating nested traps. Local macOS tests pass hover cancellation,
interactive content, tooltip/popover hit routing, accessibility exposure and
bounded timer/subscription disposal. The public Bonsai/Eio example passes retained
editor commands, controlled visibility, stale unmount and shutdown. Independent
protocol fixtures, OCaml/Rust tests, full build/format and Clippy pass locally.
No hosted or full Linux GUI acceptance is claimed for this local checkpoint;
menus/commands, feedback, pointer/desktop interactions and assets remain OCH-11 work.

Shared command registries, command buttons and single-chord shortcuts are
implemented locally. Native macOS tests pass scoped dispatch, native editing targets,
keyboard/IME priority and two-window isolation, including closing one window and
continuing in the other. Independent OCaml/Rust protocol fixtures pass. The public
Bonsai/Eio example, full OCaml build/tests/format, Rust workspace tests and
Clippy pass locally. Menu adapters are being validated locally as described below;
the command palette and other OCH-11 requirements remain In Progress.


Menus are implemented on the local OCH-11 branch: immutable command-reference
models, dropdown/context/in-window/platform presentations, virtualized cascading
popups and active-window macOS menu ownership. Targeted macOS native tests pass
actual NSMenu and accessibility activation, right-click Copy/focus restoration,
1000-entry wheel/keyboard navigation, popover integration, focused command scopes,
hidden/stale actions and menu restoration after closing a second window. The
activation test found and fixed menu ownership refresh when returning to an
unchanged surviving window. Hidden triggers now close detached popup state and
release focus. The combined native controls suite, public Bonsai/Eio example,
Rust workspace tests and Clippy pass locally; the [menu evidence report](evidence/native-menus-och11.md)
records coverage and limitations. No hosted or Linux GUI acceptance is claimed.


The command palette is implemented on the local OCH-11 branch: ordered command
references, native query/composition/history, virtualized results, shared command
execution, modal focus and accessible activation. Local macOS tests pass a
1000-command list, current-query/current-generation routing, native document Copy,
hidden/nested-modal restoration and query disposal. Visibility-driven dismissal
now runs before paint can discard focus ancestry. Full OCaml build/tests/format,
Rust workspace tests, Clippy and the public Bonsai/Eio example pass locally. The
[palette evidence report](evidence/native-palette-och11.md) records the checks and
an unresolved intermittent tooltip-hover failure seen in an earlier combined run;
the subsequent combined controls run passed. OCH-11 remains In Progress, with no
hosted or Linux GUI acceptance claimed for this checkpoint.


Progress indicators are implemented locally for OCH-11 with validated fractions,
explicit indeterminate state, native animation, percentage accessibility and the
existing root-style/theme API. Actual macOS tests pass painted dimensions/colors,
noninteractive focus, animation without OCaml commits, and hidden/determinate/
unmount cleanup. The public Bonsai/Eio example, combined controls suite, full
OCaml build/tests/format, Rust workspace tests and Clippy pass locally. The
[progress evidence report](evidence/native-progress-och11.md) records the scope;
in-app notifications are described below and OCH-12 still owns general motion
and reduced-motion integration. No hosted or Linux GUI acceptance is claimed.


In-app notifications are implemented on the local OCH-11 branch with keyed
terminal sessions, bounded stacks, explicit overflow, native active-time deadlines
and hover/focus/hidden/modal pause. Local macOS tests pass close/accessibility/
keyboard actions, native editor Escape priority, ordinary action content, expiry
without OCaml commits and unmount cancellation. The public Bonsai/Eio example
passes native dismissal delivery and keyed removal. Independent protocol fixtures,
OCaml expect tests, Rust workspace tests and the combined native controls suite
pass locally. The [notification evidence report](evidence/native-toasts-och11.md)
records exact coverage and remaining validation. No hosted or Linux GUI acceptance
is claimed. OCH-11 still includes pointer capture/drag-drop/file dialogs, assets/
images/SVG/cache and theme-scale integration, remaining state/transition work,
scrolling/lifetime checks, documentation and consolidated CI/merge.

Captured pointer regions are implemented locally for OCH-11. Real macOS native
tests pass out-of-bounds movement, redraw/reposition retention, cancellation,
modal gating, nested ownership, native child-control precedence and pressed
styling. The combined controls suite passes after final release-order review;
Clippy and the full Dune build/tests/format also pass. Independent protocol/Core
tests cover validation, callback lifetimes and motion coalescing. The public
Bonsai/Eio resize example passes its lifecycle self-test. See the
[pointer evidence report](evidence/native-pointer-och11.md) for precise coverage.
Pointer capture remains distinct from drag/drop and file dialogs, which are still
pending alongside assets/images/SVG/cache, theme-scale integration, remaining
state/transitions, scrolling/lifetime checks and consolidated CI/merge. No hosted
or Linux GUI acceptance is claimed for this checkpoint.
