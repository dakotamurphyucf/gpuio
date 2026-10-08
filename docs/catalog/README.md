# Pinned component catalog

OCH-41 is building the implementation coverage ledger and public gallery. The
files currently here establish reproducible source inputs; they do **not** claim
that every source entry is implemented or validated.

- `sources/manifest.json`: exact upstream revisions, source paths and SHA-256
  hashes. Snapshots are unmodified Git blobs; upstream licenses are alongside.
  The chart/plot, animation/motion and command/menu inputs now include 51 further
  blobs verified against the pinned Git tree. The motion, chart and command/menu reviews below now map their nested
  surfaces and remaining gaps. Source snapshots are not component acceptance.
- `inventory.json`: root modules from both Longbridge layers, GPUIX intrinsic
  elements, React export modules, style fields and all generic event properties.
  Private root modules are included deliberately so review must account for
  helpers/infrastructure as well as user-facing families.
- [Gallery contract](../design/component-gallery.md): public application design
  and remaining behavioral acceptance.
- [Adapter author and maintenance guide](../component-adapters.md): ownership,
  validation, styled-layer compatibility, source reconstruction and licenses.
- `gpuix-styles.json`: all 73 field-to-public-API mappings, with evidence entry
  points and explicit value/behavior reviews still pending. The audit checks
  completeness and that referenced APIs/files exist, not visual equivalence.
- `gpuix-values.json`: seven reviewed source-value sets: cursor, text overflow,
  gradient color space, both grid minima, whitespace and text decoration. The
  check requires complete mappings to public constructors for those declarations.
  The [finite-value audit](../evidence/style-finite-values-och41.md) records grid
  count and decoration replacement differences, paired field-byte tests and
  native refinement/transaction validation. Native GPUIX renderer/style snapshots
  support further audits of aliases and reset behavior. Other value sets and
  component-root behavior remain under review; physical OS cursor glyphs are not
  asserted by these checks.
- `gpuix-native-values.json`: fourteen additional renderer-derived keyword sets,
  with typed aliases, Boolean selection policy and a state-local unset operation.
  The [native alias audit](../evidence/style-native-aliases-och41.md) follows the
  exact GPUIX GPUI submodule helpers; it distinguishes flex-relative alignment
  from justification, normal versus base-style replacement, and fixed/absolute.
  Pointer eligibility and occlusion remain explicitly different API concepts.
- [Numeric and shorthand policies](../evidence/style-numeric-policies-och41.md):
  validated limits, units, signed/Auto choices, text byte counts and ordered
  shorthand composition, including deliberate differences from permissive
  upstream parsing/clamping. Independent OCaml/native boundary and atomicity
  checks supplement the existing codecs; specialized-root behavior stays open.
- [Native appearance observation](../evidence/window-appearance-och41.md):
  snapshots preserve all four native variants through existing window events.
  The gallery offers per-window Follow system and explicit Light/Dark palettes.
  Local checks and a real two-window macOS Light/Dark walkthrough pass, including
  independent overrides, retained editing and OS preference restoration. Other
  appearance modes and consolidated release acceptance remain separate.
- [Style/theme source review](style-theme-review.md): all nested theme helpers,
  semantic and legacy configuration, styling/sizing differences, embedded palette
  data, runtime registry behavior and font fallback boundaries. Default monospace
  selection now has [local evidence](../evidence/default-fonts-och41.md). The [file-backed gallery example](../evidence/gallery-theme-files-och41.md) now has
  parser/Eio/reconciliation evidence plus a twelve-case native file-picker,
  reload/palette, editor retention and cancellation walkthrough. The [native scrollbar snapshot](../evidence/scrollbar-preference-och41.md)
  supplies an explicit gallery action with Linux Unsupported handling. Physical
  gallery qualification remains open; no
  upstream JSON-schema or pixel-preset compatibility is implied.
- `gpuix-events.json`: all 22 event properties, with current public contracts,
  exact pinned implementation links, remaining differences, owner and platform
  limits. General input observations now have local native and public gallery
  evidence; subtree highlighting and per-file diff controls remain required v1
  work. A mapped row is not release acceptance.
  The [input-observation contract](../design/input-observations.md) and validated
  `Input_region` domain/codecs, Core/Bonsai constructor and native event delivery
  now have local native edge-case and public gallery evidence. Consolidated
  consumer/release checks remain; see the [input evidence](../evidence/input-observations-och41.md).
- `families.json`: all 146 root-module entries mapped to 43 owning families,
  public interfaces, existing examples, evidence and release scopes. Helpers map
  to their owning runtime/style/interaction contract rather than separate widgets.
  Nullable gallery pages and explicit review-pending status show remaining work;
  no module is silently dropped. Nested public families/configuration still need
  detailed audit. The display-document audit below now enumerates concrete plugin,
  configuration and styling gaps; code-editor/LSP remains explicitly post-v1.

- [Display-document review](documents-review.md): nine exact Base/component
  sources map reader formats, selection format, clamping, internal styles,
  frontmatter/MDX and block/inline plugins. Existing Markdown/code/diff behavior
  has local evidence. Static document profiles, renderer slots and application
  defaults now have public APIs and installed-consumer evidence; the
  [native profile walkthrough](../evidence/document-profile-macos-och41.md) and
  [plugin-owned scrolling](../evidence/document-profile-scroll-och41.md) cover
  keyboard/pointer navigation, updates, remount and source retirement. Rich
  Markdown AX character geometry and VoiceOver remain separate release work;
  document customization does not inherit the code-editor/LSP deferral.

- [Presentation behavior review](presentation-review.md): exact status-bar,
  badge and label source snapshots. Three-region status composition now has
  public native geometry/keyboard/identity evidence; overlay badges now add
  count/dot/icon anchoring, uncapped labels and native input/cleanup checks.
  Label configuration gaps remain explicit. This begins detailed review without
  converting the entire family into an acceptance claim.

- [Progress behavior review](progress-review.md): linear/circular and rich-center
  behavior, native value transitions and inert artwork. Local implementation and
  scoped geometry/lifecycle checks exist; physical acceptance remains open.
- [Loading behavior review](loading-review.md): spinner/skeleton/glyph-shimmer
  source mapping, custom spinner icon/easing implementation and a repaired inert
  artwork regression. Scoped clock/paint policy checks are separate from native
  GPU, accessibility and release acceptance.
- [Slider behavior review](slider-review.md): pinned base/styled API mapping,
  expanded single/range/axis/logarithmic gallery, reversed fill and per-part
  appearance. Native hover/pressed springs have lifecycle checks; physical acceptance remains open.
- [Rating behavior review](rating-review.md): pinned integer-rating behavior,
  controlled request mapping and explicit click/hover/size differences. Independent
  active/outline appearance is implemented locally; new native/gallery acceptance
  remains open. The family is not yet fully accepted.

- [Selection behavior review](selection-review.md): nine pinned checkbox,
  switch, radio and toggle snapshots; command-based single/multiple toggle
  composition, typed toolbar orientation and a mixed-checkbox gallery preview.
  Rich labels, per-part styling, independent child loading/disabled state and
  partial bulk selection are now implemented locally. Model/codec/native checks
  pass; desktop composition validation remains open.

- `expanded-v1.json`: canvas, native extensions, container rules, desktop
  integration and OS notifications from the accepted v1 expansion. These additions
  sit outside the Longbridge root-module map; they must not disappear merely
  because that source exports no corresponding module. Motion is already in the
  family map. Gallery links and pending behavioral review remain explicit. Reference-app
  consumer/distribution gates stay in the milestone release evidence.

- [Calendar and color-picker review](calendar-color-review.md): exact pinned
  sources, guarded date presets and deliberate explicit-confirm semantics.
  Multi-month calendars and internal appearance now have local integration evidence.
  Rich picker triggers now have driver evidence; native palette hover has local
  hit-test/draft/lifecycle evidence. Shared popover expanded/dialog-popup state
  has native metadata/focus/gating/nesting/cleanup checks. Per-date/header composition now has checked passive slots, atomic admission,
  native focus/lifetime and pending-confirmation driver checks plus gallery event
  badges. Independent [calendar viewport observations](../design/calendar-viewport.md)
  now drive bounded date loading without changing selection revisions or polling.
  Physical validation remains open. Grouped/featured palettes, internal
  color appearance and native Palette/HSLA tabs now have local codec/admission/
  ownership/layout evidence and public gallery examples; physical acceptance
  remains open.

- [Journey/workspace review](journey-workspace-review.md): seventeen exact
  snapshots map sidebar activation/collapse, native navigation history, carousel
  tracks, styled tabs and resizable groups. Optional sidebar selection activation
  is implemented with local reducer/reconciliation evidence. Per-label styling has
  [local composition/native evidence](../evidence/sidebar-styling-och41.md). Measured partial
  carousel tracks, rich/overflow tabs and split group/handle customization now
  have public APIs, gallery examples and local native evidence. Physical and
  release qualification remain explicit v1 work, independent of the separate
  comprehensive-docking deferral. See the review for current feature evidence.

- [Notification review](notification-review.md): exact Base/component snapshots
  map existing keyed toasts and OS delivery. Eight-anchor placement and window
  margins and measured layered expansion now have public APIs, gallery controls
  and local native evidence. Coordinated native enter/exit/reflow now has public Motion metadata, lifecycle
  tests and a gallery toggle; physical/release qualification remains open.

- [Collections review](collections-review.md): exact list/scroll/tree/table
  snapshots map existing managed behavior. General horizontal virtualization and
  shared scrollbar presentation now have public APIs and local integration evidence;
  standalone searchable lists, structural tables, retained rich headers and checked
  header/row styling now have public gallery examples and local behavior evidence.
  Tree context actions already use
  scoped commands and retained row content. Physical/release acceptance stays open.

Verify structural inventory without an upstream checkout or network access:

```sh
python3 scripts/audit_component_catalog.py
```

`--write` regenerates the structural inventory after an intentional, reviewed
source update. It does not update GPUIO's dependency pins or bless parity. Review
nested public families and configuration values as well as root names. For
example, an upstream `input` module includes more than a single text field, and
matching a style field name does not prove support for every value of that field.

The original research inventory omitted `onVisibleRange` and `onHighlight`;
both are in this source-derived inventory and event audit. The value ledger now
covers `ellipsis-start`, cursor variants and nested sRGB/Oklab gradient color
spaces; continue reviewing selection/inheritance and hit-testing semantics rather than equating field
presence with behavior. The completed ledger must link each capability to public API,
commands/events, style/accessibility behavior, runnable examples, owner, evidence
and platform status, with deferred/unsupported entries explicit.

Longbridge GPUI Kit is pinned at
`84f57fdfcb4910623fb0bb7f795b077e249f9271`; GPUIX is pinned at
`18e695ed0ee8121a7793413ca795e08eda2a13df`. These documentation snapshots are not
compiled dependencies. Actual native dependency provenance stays in
`third_party/sources.json` and the adapter reconstruction records.

- [Avatar source review](avatar-review.md): existing leaf evidence and explicit
  AvatarGroup, custom fallback content and identity palettes. These now have local
  implementation and scoped tests; actual GPU/AX/public-consumer acceptance remains open.

- [Choice behavior review](choice-review.md): eleven exact Select/Combobox and
  shared searchable-list snapshots expose the distinction between the current
  single-ID editable control and the source's popup multi-picker. The additive
  `Choice_picker` now implements multi-selection, popup search, grouping and
  custom slots with local test and gallery-build evidence. Its
  [contract](../design/choice-picker.md) preserves native ownership and
  asynchronous current-model decisions. The [physical picker follow-up](../evidence/choice-picker-macos-och41.md)
  passes native selection, Escape, search retention and accessible empty/create
  behavior. Search uses AXValue; physical typing/IME and full release acceptance
  are not inferred from that check.

- [Plain input/text-area review](editor-review.md): fourteen exact snapshots
  distinguish baseline editing from the required expanded plain-input surface.
  Password privacy and bound native edit menus now have public APIs/gallery
  examples and local behavioral checks. Format masks/validation, adornments/content hints, multiline layout/viewport
  commands and search/replacement now have public APIs and local evidence. General
  text-range geometry and physical qualification remain open. Code-editor/LSP subfamilies remain
  separately deferred; the plan does not claim family acceptance.

- [Numeric/OTP and workflow-stepper review](numeric-review.md): exact pinned
  behavior, explicit policy/presentation gaps and the corrected workflow-stepper
  ownership. Numeric increment/decrement is not workflow-stage navigation.
- [OTP cell presentation](../design/otp-presentation.md): retained grouped cells,
  bounded geometry and theme colors. [Local evidence](../evidence/otp-presentation-och41.md)
  covers codec/admission and TestPlatform state/paint checks.
  [Native caret timing](../evidence/otp-caret-och41.md) now covers bounded timer
  ownership and fake-clock lifecycle; physical macOS qualification remains open.

- [Numeric frame presentation](../design/number-presentation.md): retained editor,
  integrated adornments, custom step content and per-part paint/geometry.
  [Local evidence](../evidence/number-presentation-och41.md) covers composition,
  history, repeat cancellation and policy-gated auxiliary actions. Application-selected
  steps are implemented separately through [guarded step requests](../design/number-step-requests.md);
  physical desktop qualification remains open.


- [Overlay/help-surface review](overlay-review.md): 22 exact pinned snapshots map
  dialogs, alert dialogs, sheets, popovers, focus/positioning helpers, tooltips and
  hover cards. It distinguishes functioning renderer behavior from an unused
  sheet resize flag and a private animated dropdown helper. Modal backdrop color
  and exported modal semantics have local codec/admission/native/Core tests.
  [Opt-in modal entry](../design/overlay-motion.md) now adds dialog/sheet motion,
  with actual deferred paint/hit/AX/focus/lifecycle regressions and an installed
  gallery consumer build. [Tooltip entry/switching](../design/tooltip-motion.md)
  now has native paint/input/identity/controlled-state/cleanup and codec evidence.
  Placement/inset differences remain explicit; physical release acceptance is
  still open.

- [Tab motion](../design/tab-motion.md) adds optional indicator springs and Pill
  foreground fading; the [local evidence](../evidence/tab-motion-och41.md) includes
  interruption, scroll/reorder, actual native paint, reduced motion and teardown.
  The Navigation gallery exposes an animation toggle. Physical acceptance remains
  separate from TestPlatform checks.

- [Window source review](window-review.md): exact title-bar, window-border and
  window-extension sources map standard native windows, custom chrome/gestures,
  presentation-aware controls, automatic client frames and metadata-only focused
  input discovery. Window-wide selection helpers now implement bounded text reads,
  presence checks, clear and end-drag operations. Scoped physical macOS evidence
  now covers [standard/custom lifecycle](../evidence/window-lifecycle-och41.md),
  [focused input](../evidence/window-input-query-och41.md) and
  [window-wide selection](../evidence/window-selection-och41.md). Other platform
  behavior and consolidated release acceptance remain explicitly unqualified.

- [Geometry and element helpers](geometry-review.md): three pinned sources map
  placement/edges, native layout resolution, selection scopes and child construction.
  The public measurement boundaries are explicit; there is no generic synchronous
  OCaml prepaint callback.
- [Runtime helpers](runtime-helpers-review.md): eight sources map Base globals,
  Eio delivery/cancellation, component traits, index construction and Root providers
  to their existing application/window owners.
- [Native event helpers](native-events-review.md): native axis filtering and typed
  double-click observations use existing scroll/input routes, with raw events kept
  distinct from captured gestures and committed text.
- [Diagnostics helpers](diagnostics-review.md): counters and native observation
  are mapped with their evidence limits. Window-level macOS AX hit-test forwarding
  now has exact two-window editor and [21 control/overlay point](../evidence/form-label-point-routing-och17.md)
  results. VoiceOver remains separate from these native point queries.

- [Motion source review](motion-review.md): eight exact snapshots map transitions,
  springs, timing/easing, keyframes, presence, reveal and stagger. Existing numeric
  programs and component-specific motion are distinguished from unimplemented
  timing/easing surfaces and the already accepted keyframe/object/presence exclusions.
  Native ownership and evidence limits remain explicit; this is not blanket parity.

- [Command/menu source review](commands-menus-review.md): action ownership, native
  command/palette/menu behavior, shared searchable-list boundaries and explicit
  rich-row/search-policy/OS-popup gaps. An AppKit menu bar is not an OS popup menu.
- [Chart/plot source review](charts-review.md): all seven chart families and nested
  helpers, numerical versus categorical domains, styling/stacking limitations,
  resource/selection provenance and existing staged outlier investigation.
