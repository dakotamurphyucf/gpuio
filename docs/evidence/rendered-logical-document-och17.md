# Unrealized document accessibility — OCH-17

Local macOS arm64 implementation after `b02b6079`, 2026-10-07. This extends
[native text runs and prepared mappings](rendered-text-runs-och17.md). It does not
complete Document selection, OS accessibility or milestone 07 acceptance.

## Behavior

The existing Document tree now merges actual top-level native attachments with
prepared logical content for unrealized blocks. Visible native control identities,
subtrees and actions remain intact. Logical headings, lists, table rows/cells and
links retain their semantic structure and exact text intervals. Parent intervals
are partitioned around direct-child intervals, including partial links; repeated
strings are not used as identity. An explicit work stack avoids recursion
proportional to semantic nesting.

Top-level boundary separators remain readable beside actual native blocks.
Offscreen TextRuns include Unicode and CRLF character lengths, but no invented
layout bounds or input handlers. Native-widget alternatives are readable; only
their outer edges are valid native selection positions. Empty-copy objects use
the prepared U+FFFC alternative without changing Copy text. Empty documents have
a caret run. Clamped previews retain their clipped-content policy.

The run index now orders by global accessible UTF-16 offset. This distinguishes
reading positions inside multiline atomic alternatives while keeping native
object selection atomic. These are still last-prepaint position mappings, not
authorization to act on a current native owner.

## Validation

- Native library tests: **1,140 passed, 2 existing ignored**.
- Strict all-target workspace Clippy, Rust formatting and diff checks pass.
- GPUI Base reconstructs exactly from its cached, hash-verified pinned archive:
  **243 files**, excluding Cargo.lock. Patch SHA-256:
  `c77049e5267b21408001d84f16c673df408ab6767ff4c1d04cdd348a2fcbcd66`.
- The restored session passes actual macOS native editor/document checks and
  the normal Rust workspace suite, and Dune `@all @runtest @fmt`. The rebuilt
  gallery passes native reading order, rendered Select All/Copy and normal
  shutdown; its child exits cleanly and the typed clipboard is restored.
  Those desktop checks exercise existing native
  input/Copy/rendering behavior; they do not qualify full rendered AX selection
  or VoiceOver. No current-source Linux or hosted acceptance is implied.

The semantic tests compare actual tree reading order against the full prepared
text over 200 virtualized paragraphs, exercise every ordinary scalar boundary,
and require actual native attachments only for realized blocks. They cover
replacement/resource refresh, foreign windows, stable control IDs and actions,
nested offscreen headings/lists/tables/links, exact partial-link text, CRLF atomic
alternatives, empty objects, an empty caret and clipped previews.

Initial failures are retained. The first filter compiled but selected zero tests;
it is not test acceptance. Complete-text assertions then found missing boundary
newlines around visible paragraphs; the implementation now publishes those from
prepared separator provenance. A pointer test previously equated AX text presence
with native realization. It now asserts actual attachment retirement and absence
of layout bounds while preserving its existing selection/direction requirements.

The [report archive](rendered-logical-document-och17/reports.tar.gz) and
[manifest](rendered-logical-document-och17/manifest.json) retain logs, commands,
source hashes, a changed-source snapshot and reconstruction evidence.

## Remaining requirements

Text gaps inside **realized nested** structures still require integration.
Large-tree publication cost, visual-line adjacency, final-paint selection,
guarded reveal/activation and OS selection actions remain unqualified. Offscreen
link roles and URL metadata do not implement an activation handler. Actual macOS
accessibility and VoiceOver validation remain required.

OCH-17/OCH-41 and the full release goal remain open, including catalog,
performance/resources, distribution and required current-source Linux checks.
The owner restored unrestricted access after the restricted-session failures.
OCH-17 progress was published as comment `c77453df-663d-4463-b66c-dab4e1408c5f`.
Final source delivery, review and release acceptance remain separate requirements.
