# GPUIO macOS accessibility adaptation

This is the published `accesskit_macos` **0.26.3**, at AccessKit revision
`c88605b96d04431f9c3c792464a0f2f253480e94`, with nine scoped patches.
The upstream MIT/Apache-2.0 notices and both license texts are preserved. Source,
archive checksum and original per-file checksums are recorded in `UPSTREAM.json`.
Cargo uses the registry-normalized manifest, retaining its exact dependency ranges.
No GPUI/AccessKit version upgrade is involved.

The adapter's original `PlatformNode` did not implement `isAccessibilityExpanded`
or allow that selector, even for AccessKit nodes with `expanded = Some(true)`.
Consequently a GPUI button carrying `aria_expanded(true)` reported false through
AppKit. This was reproduced by GPUIO's actual native disclosure test.

`expanded-state.patch` adds a read-only expanded getter, advertises the selector
only for nodes that carry expanded state, and posts a value-changed notification
when expansion changes independently of the node's ordinary value. Activation
continues through existing accessibility Press and the asynchronous application
intent bridge. It does not add a second state owner or synthesize toggle values.

`tree-state.patch` adds the outline-row `isAccessibilityDisclosed` getter and
`accessibilityDisclosureLevel`, with selector availability restricted to TreeItem
nodes carrying the respective metadata. AccessKit's one-based level becomes the
outline row's zero-based indentation. The native GPUIO tree test reproduced a
child row reporting indentation zero and no disclosure getter before this patch.
There is no dependency/version change or second expansion owner.

Apple documents [outline-row disclosure](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol/setaccessibilitydisclosed(_:))
and [disclosure level as row indentation](https://developer.apple.com/documentation/appkit/nsaccessibilitydisclosurelevelattribute).
The existing generic expanded getter remains for disclosure buttons; leaves do
not advertise a disclosure state. Notifications continue through the existing
expanded-state value-change path. No VoiceOver speech or external AX notification
observer acceptance is claimed by these getter tests.

`tree-actions.patch` adds per-row `setAccessibilityExpanded:` and
`setAccessibilityDisclosed:` using the existing Expand/Collapse actions. Only
enabled TreeItems with expanded metadata and both handlers advertise the setters.
It also adapts `setAccessibilitySelected:` for opted-in GPUIO TreeItems. AccessKit
0.24.1 has no desired-selection action, so these rows declare two CustomActions:
`0x47500001` (select) and `0x47500002` (deselect). The adapter requires both IDs,
the CustomAction handler and an enabled selectable TreeItem. Other roles and
unrelated TreeItems retain upstream selection/click behavior. GPUIO reduces the
desired membership against current Core state without implicit focus/activation.

Setters queue the requested state even when the displayed snapshot already has
that value: an earlier opposite setter can still be waiting for asynchronous
reduction. Deduplicating against the displayed state would lose ordered
select/deselect or expand/collapse sequences. Native/Core lifetime, disabled and
identity checks remain authoritative; these actions introduce no native preference
state. Actual AppKit tests cover mixed setters before the next application update.

Reconstruction: download the crate archive from `UPSTREAM.json`, verify its SHA256,
extract `Cargo.toml`, `Cargo.toml.orig`, README/CHANGELOG and `src/`, then apply
`patch -p1 < expanded-state.patch` and then `patch -p1 < tree-state.patch` inside
that directory, followed by `patch -p1 < tree-actions.patch`, `patch -p1 < table-state.patch`
and `patch -p1 < document-semantics.patch`, then `patch -p1 < table-headers.patch`
and `patch -p1 < initial-window-focus.patch`, then `patch -p1 < busy-state.patch` and `patch -p1 < list-selection.patch`.
Fetch LICENSE-APACHE and
LICENSE-MIT from the pinned upstream Git revision and verify their recorded hashes.
`UPSTREAM.json`, this note and the patches are GPUIO provenance additions. The
registry archive's Cargo.lock and Cargo cache metadata are not build inputs.

The root workspace and generated static extension backend both carry the same
`[patch.crates-io]` path. Dune explicitly tracks this source tree. Custom backend
workspaces must propagate that patch: dependency manifests cannot impose a Cargo
patch on their consumers. `scripts/compose_backend.py` does this automatically.

Validation lives in `rust/native/src/navigation_test.rs` and `tree_view_test.rs`:
the tests query actual AppKit expanded/outline state before and after collapse;
the navigation test also invokes native Press. The
standard native controls/tabs regressions continue to cover other adapter roles.
No VoiceOver speech or external AX notification-observer acceptance is claimed by
the getter test. Remove this fork when an evaluated pinned upstream adapter
provides the same behavior and the native regression passes without it.

`table-state.patch` exposes logical row/column counts for semantic Table nodes,
zero-based row indices and row/cell/header index ranges, header sort direction,
and mounted table rows/selected rows. It never synthesizes the full logical
history. Row enumeration skips nested tables/trees. AccessKit data indices and
counts are preserved directly; the data-row count excludes visual headers.

GPUIO table Row/Cell/ColumnHeader nodes opt into desired selection using custom
IDs `0x47500011` (select) and `0x47500012` (deselect). Both declarations and the
CustomAction handler are required, preserving the separate existing TreeItem
contract. Ordinary Press remains a distinct action. The native table reducer
updates optimistic selection immediately, and queues typed application requests;
deselection of an unrelated target cannot clear another selection. Header sorting
also has a separately labelled native Button for Cocoa Press; sort indicators
reflect native optimistic state until the application updates/reset columns.

`rust/native/src/table_host_test/accessibility.rs` queries real AppKit counts,
ranges, Unicode cell values, selected rows, sort direction and focus, and invokes
Press, focus and desired-selection setters. It verifies queued order, a jump to
row 50,001, unavailable data, pointer-independent accessibility and hidden/disabled
retirement. Focus assertions activate the local test window; the harness closes
it on success and failure. This is not VoiceOver speech or Linux GUI evidence.

`document-semantics.patch` corrects the Heading role string to `AXHeading`
(the same native role already used for DocSubtitle) and implements the legacy
AppKit AXValue-settable query using the adapter's existing SetValue capability
predicate. On macOS 14.5, external AX queries reported a read-only code document
as settable even while `isAccessibilitySelectorAllowed:` returned false with
read_only=true, text_ranges=false and no SetValue action. The narrow override
fixes that mismatch and delegates other attribute queries to AppKit.

The gallery's real macOS regression checks Markdown heading/list roles and
Unicode body text, code/diff values, AXValue not settable, native focus, rejection
of typing/backspace, collapse and remount. Existing editable fields must still
accept AXValue replacement. The patch now maps a heading's AccessKit level to its
numeric AXValue, following [WebKit's macOS heading value mapping](https://chromium.googlesource.com/external/Webkit/+/b4170928e42cb313b7c8304a796879ddb2ff7f12/Source/WebCore/accessibility/mac/WebAccessibilityObjectWrapperMac.mm).
The painted children continue to supply its text. The external gallery regression
reads level 1 and checks table row/cell indices through the existing table adapter.
Ordinary link actions have separate passing evidence; complete rich-link behavior,
selection ranges and VoiceOver reading remain release work. The expected native heading role is also
described in [WebKit's heading mapping](https://bugs.webkit.org/show_bug.cgi?id=131920).

`table-headers.patch` exposes the table's current column/row header nodes through
`accessibilityColumnHeaderUIElements` and `accessibilityRowHeaderUIElements`.
`accessibilityHeader` returns the nearest shared exposed row/group ancestor of
the column headers, if one exists below the table. See [Apple's header API](https://developer.apple.com/documentation/appkit/nsaccessibility-c.protocol/accessibilitycolumnheaderuielements)
and the [Core AAM 1.2 draft table mapping](https://www.w3.org/TR/2026/CRD-core-aam-1.2-20260923/#role-map-table).
Getters traverse only the current filtered tree, reject hidden/retired tables,
and stop at nested tables/grids/trees. Returned objects reuse the adapter's
existing node identities; no synthetic header copies, offscreen materialization
or new persistent caches are introduced. Column indices correlate these headers
with cells. This does not implement an AXColumns object model or prove VoiceOver
column navigation.

The external gallery checks Markdown headers against the actual first-row cells
and AXHeader against that row, including collapse/remount. The native 100k-row
table regression checks its two painted headers and shared group, repeats after
scrolling to logical row 50,001, and verifies retained hidden-table references no
longer expose headers. The managed table now marks its existing header container
as RowGroup while preserving the delegate's element identity. Empty row-header
arrays are correct for these fixtures; row-selection buttons are not row headers.

`initial-window-focus.patch` seeds the window adapter's host-focus state from
`NSWindow.isKeyWindow` at installation. The pinned GPUI macOS backend can install
the adapter after AppKit has already made a newly opened window key. Starting
unconditionally unfocused then suppresses the accessible focused node until a
later key-window transition, although keyboard editing already works. The
generic view constructor retains its documented before-first-focus behavior.
Subsequent focus updates, accessibility tree ownership and action routing are
unchanged; the patch does not activate windows or synthesize focus events.

The public gallery's Settings two-window regression checks initial `AXFocused`,
inactive peer state, switching back, actual OS edits, independent resets and the
surviving window after close. These external getter/input checks do not establish
VoiceOver speech or focus-notification delivery. No dependency version changes.


`busy-state.patch` exposes AccessKit `is_busy` through the legacy
`AXElementBusy` Boolean attribute. AppKit has no public modern busy getter in the
pinned SDK. Attribute enumeration extends the superclass list and preserves the
existing optional Braille attributes; other legacy getters delegate to AppKit.
The attribute is read-only because application state owns loading. Live ready
nodes return false, busy nodes return true, and retired nodes return no value.
The existing change queue receives `AXElementBusyChanged` only for included-node
busy transitions, independently of ordinary value changes. It adds no polling,
callback into OCaml or second state owner.

Sources: Apple's [busy attribute](https://developer.apple.com/documentation/applicationservices/kaxelementbusyattribute)
and [busy notification](https://developer.apple.com/documentation/applicationservices/kaxelementbusychangednotification),
plus the installed SDK's `AXAttributeConstants.h` and `AXNotificationConstants.h`.
A headless main-thread AppKit fixture (`rust/native/tests/accessibility_busy.rs`)
checks actual adapter objects through Objective-C attribute selectors without an
NSApplication or window. These getters are distinct from external AX notification
receipt and VoiceOver behavior, which remain physical desktop acceptance gates.
The public gallery drivers now require external AXElementBusy checks but are
unrun at this checkpoint.

Reproduce the source reconstruction without changing the repository or accessing
the network (download the exact `source` URL in UPSTREAM.json separately):

```sh
python3 scripts/verify_accesskit_macos.py --archive /path/to/accesskit_macos-0.26.3.crate
```

The verifier checks the archive and every original source hash, applies all nine
patches without offset/fuzz, compares every reconstructed source byte and verifies
the two license hashes. Its temporary workspace is removed on completion.

`list-selection.patch` extends ordered desired-selection setters to opted-in
ListBoxOption nodes. Both custom IDs `0x47530001` (select) and `0x47530002`
(deselect), a CustomAction handler and an enabled selectable option are required.
Each setter queues its desired value, including values equal to the displayed
snapshot, because preceding setters may still await OCaml reduction. Focus,
confirmation and context requests stay distinct; no native selection state is added.
Other roles and options retain their existing behavior.

`rust/native/tests/accessibility_list_selection.rs` reproduces the lost ordered
setters before this patch using actual AppKit adapter objects on an NSView with
no OS window. It checks ordered actions, unchanged selection snapshots, exact
role/action opt-in and disabled/removal/adapter retirement. This is not external
AX notification or VoiceOver acceptance.
