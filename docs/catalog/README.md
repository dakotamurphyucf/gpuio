# Pinned component catalog

OCH-41 is building the implementation coverage ledger and public gallery. The
files currently here establish reproducible source inputs; they do **not** claim
that every source entry is implemented or validated.

- `sources/manifest.json`: exact upstream revisions, source paths and SHA-256
  hashes. Snapshots are unmodified Git blobs; upstream licenses are alongside.
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
- `gpuix-native-values.json`: nine additional renderer-derived keyword sets,
  with typed aliases, Boolean selection policy and a state-local unset operation.
  The [native alias audit](../evidence/style-native-aliases-och41.md) follows the
  exact GPUIX GPUI submodule helpers; it distinguishes flex-relative alignment
  from justification, normal versus base-style replacement, and fixed/absolute.
  Pointer eligibility and occlusion remain explicitly different API concepts.
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
  detailed audit, including code-editor and document plugin subfamilies.

- `expanded-v1.json`: canvas, native extensions, container rules, desktop
  integration and OS notifications from the accepted v1 expansion. These additions
  sit outside the Longbridge root-module map; they must not disappear merely
  because that source exports no corresponding module. Motion is already in the
  family map. Gallery links and pending behavioral review remain explicit. Reference-app
  consumer/distribution gates stay in the milestone release evidence.

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
