# Settings application composition

OCH-41 work in progress. The bounded Core catalog/navigation/reset model is
implemented in [Settings](../../lib/core/settings.mli), with a controlled
[Bonsai composite](../../lib/bonsai/settings_panel.mli). Deterministic composition
and lifetime checks pass. Typed field helpers and editor mount-seed primitives
are implemented. Complete field integration, the public gallery and actual
native/installed-consumer acceptance remain required; this is not yet a
functional-equivalent catalog claim.

## Source and ownership

The pinned [settings module](../catalog/sources/component-setting-mod.rs.txt)
composes searchable pages, virtualized groups, fields, reset controls and a
resizable sidebar. Its [page](../catalog/sources/component-setting-page.rs.txt),
[group](../catalog/sources/component-setting-group.rs.txt),
[item](../catalog/sources/component-setting-item.rs.txt) and
[field](../catalog/sources/component-setting-fields-mod.rs.txt) snapshots are
catalog inputs. Rendering/get/set closures in those Rust types are not an FFI
contract.

GPUIO separates three responsibilities:

1. `Gpuio.Settings` owns immutable descriptive metadata and navigation intent.
   It contains no callbacks, native handles, values, tasks or persistence.
2. The Bonsai composite owns the bounded active group presentation,
   sidebar requests, search presentation and native layout observations. Application
   field values/controllers and asynchronous tasks remain outside transient rows.
3. Existing native input controllers own editing, composition, selection and undo.
   Eio application services own loading/saving and report errors through application
   state. Rendering and filtering never read a file or implicitly save a setting.

## Implemented identity and filtering contract

Page, group and item IDs are distinct OCaml types. Each is case-sensitive,
1..256 UTF-8 bytes without NUL. IDs are unique across the entire catalog within
their kind, so filtering, reordering and group virtualization cannot turn a list
position into another field's identity.

Normal item search matches the title, optional description and explicit keywords.
Custom items match keywords only; empty search includes them. Page/group headings
do not implicitly make all their descendants match. Matching uses Unicode
lowercase substrings, independently per field, with no trimming, accent removal,
normalization or locale-specific collation. This follows the pinned source's
item-search structure; it is not a fuzzy search engine.

Catalog bounds are 128 pages, 2048 groups, 32768 items and 8 MiB of text including
IDs and cached search text. Query input is at most 1024 bytes. Titles are bounded
to 4096 bytes, descriptions to 16384, keywords to 64 × 256, and an item's cached
lowercase fields to 64 KiB. UTF-8/NUL and bounds are checked before admission.
Aggregate counts are checked before flattening/index allocation. These bounds
are admission policy, not measured application performance or total RSS.

Filtering is cached with the immutable model and preserves source order. It
omits groups/pages with no matching items. `preferred_selection` records the
user/application's identity preference; `selection` is the effective visible
selection. If its page survives, a hidden group selection temporarily falls back
to that page. If the page is absent from the filtered result, the first matching
page is shown. No match yields None. Clearing search restores the preference.

`with_pages` validates replacement metadata atomically. It preserves a surviving
page/group preference, downgrades a removed group to its page, clears a removed
page preference and retains surviving page expansion. `default_open` applies
only to new pages. Requests for removed or nonmatching destinations are ignored
when reduced against current metadata. Programmatic selection can deliberately
name a filtered-out destination, which becomes effective when it matches again.

## Implemented reset intent contract

Each item reports `Unavailable`, `Clean` or `Dirty`. A custom field participates
only when the application supplies that status and an eventual reset handler.
The model never infers equality between heterogeneous application values.

`reset_targets` returns item IDs in catalog order:

- `Matching_page page_id` covers matching items in that effective page. A delayed
  request for an old page returns an empty list rather than resetting the new page.
- `Matching_group group_id` covers matching items in that group of the effective
  page.
- `Whole_page page_id` explicitly includes filtered-out items in the named page.
- `Item item_id` explicitly addresses one field, independently of search.

Every scope excludes disabled, clean and unavailable fields and respects the
page's `resettable` policy. Missing destinations return an empty list. This is
an intentional, explicit policy instead of inheriting the source's filtered-clone
reset loop or treating disabled presentation as permission to mutate a value.

Reset handlers must resolve the captured scope against the latest model after
asynchronous confirmation, rather than replaying an old list of setters. Native
editor changes still require the existing lease/revision checks. A pure reset
plan does not promise atomic persistence across multiple settings, guarantee a
native edit will succeed, or overwrite in-flight composition. The forthcoming
adapter must make those outcomes visible and test them.

## Implemented controlled composition

`Gpuio_bonsai.Settings.component` takes reactive metadata, appearance, a supplied
search editor view, request/reset callbacks and an item renderer. It returns a
view and active-group/budget observations, or a presentation error. It does not
create an editor or infer application value types. Reducing a request and actually
performing a reset remain application responsibilities.

The sidebar uses ordinary disclosure and composed-link controls. Page and group
IDs live in separate key namespaces, including full-length IDs and names that
coincide with internal slot names. Page icons use passive composed-link content;
rich page suffixes may contain ordinary controls. Current page/group destinations
carry localized accessibility descriptions. Search, empty state, resize and reset
labels are explicit. Invalid localized labels or unsupported icon contents are
presentation errors.

The native split defaults to a 250px sidebar with a 160..360px range. The caller
must supply bounded dimensions. Groups use the managed native list, with a 200px
estimated height, 400px overscan and 32 active groups by default; callers can
replace these configurations. Native group reveal uses stable IDs. Repeated
activation of the current group requests another reveal. Leaving a page or
filtering out all its content retires its transient list and item computations;
revisiting creates a fresh visit. Application values, editors and tasks must remain
outside these computations. A page change starts a fresh viewport rather than
implicitly persisting page-specific scroll positions.

Keeping a controller alive does not by itself preserve an unmounted native editor
session. The text/number controllers now expose explicit mount-seed overrides on
`view`: `initial_text` and `initial`, respectively. Omitting the override keeps
the original creation seed. Supplying current application data allows a later
placement to start with that data, without replacing a live draft during ordinary
observations. Numeric `initial` seeds the committed value; preserving a distinct
unfinished numeric draft across unmount remains adapter/native work. Native undo
history, selection and IME state must not be described as surviving destruction.
Focus/composition pins and the complete field-remount policy still need Settings
gallery coverage.

Sidebar-only changes preserve the group collection. With an unchanged group
order, metadata changes update only affected collection entries and their height
invalidations. Structural membership/order changes rebuild collection metadata.
This is bounded composition behavior, not measured native scroll performance.

A native width observer has empty breakpoint branches; it changes styles on one
stable set of controls. Below 480 logical pixels the field layout is vertical;
at or above 480 it is horizontal. This exact boundary is an explicit GPUIO choice
(the pinned source treats 480 itself as vertical). An item can force vertical
layout at any width. Group size controls spacing, and the chosen appearance and
group variant feed the existing group-box adapter. Disabled normal/custom items
wrap their entire content in native `Inert`. Native field names/help/error metadata
are still the responsibility of supplied controls and the forthcoming helpers.

The page-visit regression exposed an existing Bonsai virtual-list weakness:
captured viewport, retention and controller effects could act after revisiting
the same generation. Those effects, including tree-input delivery, now check the
managed generation's lifetime at execution. This supplements native handler
generation checks; it does not change wire formats or Rust ownership.

## Typed field helpers and editor commands

[`Gpuio.Settings_field`](../../lib/core/settings_field.mli) supplies controlled
switch/checkbox helpers, typed dropdown collections and a decorator for existing
native text/number/other form controls. The Settings panel owns the visible item
title. Field metadata names the native control and supplies semantic help/error
relationships; helpers display help/error in stable slots below that control.
Avoid repeating the same help in both item and field metadata. Size controls
spacing/font; control-specific configurations remain explicit. Horizontal helpers
default to 256px, vertical to full width, with caller style refinements.

Boolean callbacks are toggle intents reduced against current application state,
so two queued activations are not two copies of a captured negation. Typed
`Choices.create` takes a comparator and pairs application values with stable native
options. Both values and IDs must be unique; labels may repeat. Reordering or
renaming options does not redefine identity. Absent selection is supported;
foreign selected values return errors. Disabled options can stay selected but
cannot be activated natively. The ordinary `Choice.Appearance` controls popup
styles, dimensions and visible-row budget, including scrolling long option sets.

Text and numeric fields use their existing Eio controllers with `control`; there
is no new editing runtime or implicit setter. Text now exposes
`replace_if_unchanged`, matching the numeric controller's guarded replacement:
the expected snapshot identifies both native lease and revision. Explicit reset
handlers must also recheck application metadata/disabled/reset policy and report
native errors. The existing native command path rejects active composition;
this checkpoint does not add new OS IME acceptance evidence.

## Remaining field and acceptance work

The remaining implementation and native validation must use this model and
composite rather than a separate positional state machine:

- Searchable page/group sidebar, selected destinations and group reveal; native
  resizable split with source-equivalent 250px initial and 160..360px range.
- Native managed virtualization of groups, with explicit ownership of field values,
  editor sessions, active rows and reset requests. Hidden/evicted presentation must
  not cancel application work or reset stored settings.
- Horizontal/vertical field composition, a 480px content-width breakpoint,
  group variants/sizes, page icons and rich header/suffix/custom slots. Responsive
  changes must preserve the same control placement. Do not duplicate an editor
  controller into two retained container-query branches; that violates its lease.
  Use stable content plus layout observations/refinement, or an evaluated native
  primitive if existing composition cannot satisfy this invariant.
- Typed switch/checkbox, string, bounded-number, dropdown/scrollable-dropdown and
  custom field adapters using existing public controls. Expose application values
  and setters with their actual OCaml types, not a dynamic Rust type ID or a generic
  JSON value. Native dirty drafts/composition are not implicit value replacements.
- Explicit localized reset controls and scope labels; custom reset behavior;
  disabled field enforcement for both built-in and custom controls; accessible
  field labels/help/errors, keyboard navigation and focus reveal.
- A public gallery application showing real edits, search/selection recovery,
  resets, resize/virtualization and application-owned Eio save failures/recovery.

Required acceptance includes filtering/empty results, stale navigation/reset
intents, disabled/custom fields, in-flight edits and composition, control identity
across responsive changes, group eviction/remount, multiwindow isolation,
keyboard/clipboard/focus behavior, reset errors and persistence failures. The
example must also run against independently installed public packages. Source
presence, pure model tests and a screenshot cannot establish this acceptance.

## Foundation evidence

Local macOS 14.5 arm64: five Settings expect tests pass navigation/filter recovery,
Unicode and per-field matching, custom keywords, reset scope/current-state guards,
removed groups/expansion, malformed IDs, duplicate identities, foreign-group
selection, page and text budgets. Full Dune build/tests/format pass:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @test/view_api/runtest
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
```

Three Bonsai expect tests additionally cover optimized/unoptimized page visits,
stable control reconciliation across 479/480/700/320px observations, repeat group
reveal, stale reset intent, absent-result teardown/recovery, localized-label
errors, disabled custom-row styles and independent ID namespaces (including
256-byte IDs and internal slot-name collisions). A metadata change invalidates
only its group; sidebar expansion leaves height revisions unchanged. A separate
virtual-list regression exercises captured viewport/pin/controller effects after
revisiting a generation, with current observations still accepted. Full Dune
build/tests/format and the structural catalog audit pass locally.

These checks synthesize observations and inspect Bonsai/Core reconciliation; they
do not establish actual native breakpoint geometry, editor/IME preservation,
disabled OS input, accessibility-reader behavior or scrolling performance.

Three field-helper expect tests cover typed choice mapping/ambiguity, relabeling
and reordering, native dispatch policy, queued Boolean intents, semantic help/error
and editor identity across annotation/layout changes. The public text-input
example passes two actual background macOS windows (single/multiline), confirming
mount-seed changes leave live text unchanged, remount uses the supplied text,
fresh guarded resets succeed and stale revisions/leases fail. The public numeric
example passes its command/history/policy checks plus a changed mount seed leaving
the live `1e-` draft unchanged and taking effect after remount. These are native
programmatic command tests, not physical-keyboard or OS IME qualification:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec _build/default/examples/text_input/main.exe --self-test
GPUIO_JOBS=2 ./scripts/gpuio exec _build/default/examples/numeric/number.exe --self-test
```

There is no complete Settings GUI or installed-consumer acceptance yet. Settings, OCH-41
and milestone 07 remain incomplete. Required Linux non-GUI checks and OCH-17
release gates remain; full Linux desktop qualification is deferred to OCH-47.
