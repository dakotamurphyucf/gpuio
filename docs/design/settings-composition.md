# Settings application composition

OCH-41 work in progress. The bounded Core catalog/navigation/reset model is
implemented in [Settings](../../lib/core/settings.mli). This is the foundation
for a settings application, not an implemented renderer or a functional-equivalent
catalog claim. Bonsai composition, typed field adapters and public/native consumer
acceptance remain required.

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
2. The planned Bonsai composite owns the bounded active group presentation,
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

## Required composite and field work

The remaining implementation must use this model rather than a separate positional
state machine:

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

There is no Settings GUI or installed-consumer acceptance yet. Settings, OCH-41
and milestone 07 remain incomplete. Required Linux non-GUI checks and OCH-17
release gates remain; full Linux desktop qualification is deferred to OCH-47.
