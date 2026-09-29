# Composed links and native focus policy

OCH-41/OCH-17; pinned-source review at GPUIO `1785139`.
Status: **native mounting and focused macOS gallery checks implemented; broader
focus/content validation remains open**. This document does not claim link-family parity.
Existing `Presentation.link` continues to provide a text-only Link-semantic button.

## Why a native contract is needed

Pinned Longbridge sources:
[base/link](../catalog/sources/base-link.rs.txt) and
[component/link](../catalog/sources/component-link.rs.txt), revision
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The exact files and upstream license
are checked in and recorded by `docs/catalog/sources/manifest.json`.

| Surface | Existing GPUIO | Required closure |
| --- | --- | --- |
| Text, themed link styling, asynchronous activation | `Presentation.link`, ordinary `Style` states and generation-checked button delivery | Keep these behaviors and current helper compatibility. |
| Arbitrary composed visible content | Buttons only admit two decorative icon slots in both renderer and native tree validation | Add a native Link root and documented passive content policy; do not smuggle unsupported children through icon slots. |
| Target/open callback | Caller's action/effect captures its route or URL; desktop opening remains an explicit job | Preserve this ownership. No URL needs to cross the link transport merely to call back into the application. |
| Activation observer after injected open | Caller composes the application action/effect | Document ordering within that action; native delivery enqueues once and never synchronously calls OCaml. |
| Disabled interaction/style | Existing button helper fences disabled callbacks and native focus | Preserve native pointer/keyboard/AX rejection and dynamic style recovery on the new root. |
| Explicit AX label | Existing text helper permits ordinary accessibility metadata | New composed root requires a validated name, independent of visible content and target. |
| Tab stop/order | Existing helper always uses the ordinary button Tab stop/order | Add explicit native tab-stop policy and signed ordering with correct modal/inert behavior. |
| Native identity/resource retirement | Existing keyed reconciliation and native leases | Content/config changes must retain the root handle; removed descendants release resources and stale actions must be fenced. |

The old styled component stores `disabled` but does not use it, always opens its
href before its callback, and is pointer-only; its own tests document this. GPUIO
will keep its stronger disabled and keyboard behavior. The newer base link already
uses injected navigation and real keyboard/AX activation. It does not set an AX
URL, so no URL attribute parity is implied by its `href` builder.

## Configuration foundation

`Link.Config.create ~label ?disabled ?tab_stop ?tab_index ()` returns a validated,
abstract OCaml value. The label is nonblank according to Core's ASCII whitespace
set, valid UTF-8, without NUL, and at most 4096 bytes. It is an accessible name,
not rendered content. No trimming or normalization changes the caller's text.

Defaults: enabled, Tab participation true, index zero. `tab_index` is a signed
integer in -1,000,000..1,000,000. A negative index is an ordering value, not an
implicit opt-out; `tab_stop=false` explicitly excludes Tab/Shift-Tab while allowing
pointer and explicit accessibility/programmatic focus when enabled. Disabled
suppresses every activation/focus route, regardless of stored tab intent. Keeping
tab intent while disabled allows predictable re-enablement.

`Gpuio_protocol.Link_wire` and Rust `link::Config` have the same field order:
UTF-8 label, disabled Boolean, tab-stop Boolean, signed integer index. Rust's
standalone decoder bounds the declared string length before allocation, validates
Boolean tags and indices, rejects invalid UTF-8, truncation and trailing bytes,
and caps the whole frame at 4116 bytes. OCaml validates constructors and
`Link.Expert.of_wire`; generated `bin_read` alone is not an admission boundary.

Independent fixture: label `Guide 世界`, disabled false, tab-stop false, index -2:

```text
0c477569646520e4b896e7958c0000fffe
```

The original configuration-only checkpoint added no opcode, node kind or
capability. The subsequent development implementation appends Link kind 51 and
Set_link operation 60, while leaving the advertised capability mask unchanged
until the remaining native contract is validated. This development checkpoint is
not a promise of mixed-version bridge compatibility.

## Configuration validation checkpoint — 2026-09-29

Three Core expect tests pass: independent bytes/decoded-value validation, default
and all focus-flag/index combinations, malformed labels and numeric boundaries.
The full Rust protocol suite passes **231 tests**, including two new link tests
for independent bytes, every truncated prefix, invalid Boolean/UTF-8 framing,
trailing bytes, exact limits, extreme indices and declared-allocation bounds.
Source manifest/hash auditing and the full local isolated Dune
`@all @runtest @fmt` pass. Strict protocol Clippy (`--all-targets -- -D warnings`)
also passes. These data/build tests are not native link, Tab or AX
acceptance. No GUI window was opened for this configuration-only checkpoint.

## Mounted development checkpoint — 2026-09-29

`View.link` and `Gpuio_bonsai.View.link` now accept validated configuration,
passive children and one asynchronous activation. `Presentation.composed_link`
adds ordinary theme/style defaults; the original text helper stays compatible.
The native root retains its focus handle across content, config and theme changes.
Its config label supplies the accessible name; ordinary explicit accessibility
metadata can refine that name. Routing/URL opening stays in the application effect.

Descendants currently admit layout, text/styled text, image/icon, avatar, loading
and animation nodes, without callbacks. Nested links/controls, selectable text,
scrolling, inert branches and pointer shields are rejected. Selection inherited
from outside the root is suppressed for its content. Content is bounded to 4096
nodes/128 levels, additionally subject to global tree depth, transaction and
retained-memory limits. Resources use the existing scoped registration/lease
ownership. The final native tree revalidates dirty ancestors, including mutations
to child handlers/styles, and rejects invalid batches atomically.

Implemented checks:

* Six OCaml expect tests: independent config/operation/kind bytes, validated
  labels/focus intent, passive-content boundaries, current callback delivery,
  retained root identity, disabled/unmounted stale actions and idle reconciliation.
* Four Rust admission/session tests plus one operation fixture: final-tree
  rollback, forbidden deep mutations, exact width/depth limits, retained payload
  budgets, disabled delivery with a retained handler and stale generations.
* `native_link`: a real macOS window with GPUI-dispatched pointer/Return/Space,
  stable signed Tab order and ties, reverse/dynamic order, Tab opt-out, inherited
  selection suppression, focus-trap entry/restoration, hidden/inert exclusion,
  native identity and disposal. Direct AppKit AX checks verify Link labels,
  explicit focus without activation, one press and disabled focus/action rejection.
* Public `--section links` gallery: **eight theme/description/icon combinations**,
  **34 real pointer/Return/Space/AX activations**, native identity (`CFEqual`),
  forward/reverse order, disabled recovery and zero image/source registrations
  after page departure. The optional SVG and localized descriptions use public
  APIs. Both theme screenshots were produced and inspected.
* Full Dune `@all @runtest @fmt` passed before the subsequent gallery addition;
  the final gallery/link/format build and strict native/protocol all-target Clippy
  pass separately. The existing full native controls suite and slider suite also
  pass, including the latter's 1024-owner/2048-thumb disposal checks. Those slider
  checks do not yet establish mixed slider/link custom ordering; their timings
  are regression diagnostics, not the release performance acceptance workload.

The native regression exposed and fixed a redraw issue: focus records are rebuilt
every paint, but post-update focus finalization is not run every paint. Traversal
now orders its current eligible records on each Tab key, preserving paint order
for equal indices. It never temporarily focuses an ineligible control to inspect it.

### Open focus behavior: clipped controls

The first public gallery ordering run failed when only the first link was visible.
GPUIO's fallback traversal records controls intersecting the paint mask; the later
links were outside the scrolled viewport. The eight-case run above explicitly
keeps all three links visible. It establishes ordering among those painted controls,
**not offscreen traversal or automatic reveal**. Assess this against ordinary GPUI
Tab traversal and define/test a consistent scrolled-content contract before closing
the link rows. Do not silently treat a viewport workaround as feature acceptance.

Mixed native compound-control ordering, full passive animation/avatar/content
coverage, style/AX metadata refinements, pending-action races and fresh consumer/
release checks also remain. No Link capability bit is advertised yet.

## Full implementation and acceptance contract

1. **Public view and styled helper.** Draft the composed `View.link` and
   `Presentation` interface around `Link.Config` and ordinary view children.
   Keep target routing inside the application effect. Return a typed construction
   error for unsupported content rather than allowing a malformed native tree.
   Preserve the simple existing text helper and its asynchronous effect semantics.
2. **Content ownership and semantics.** The root owns one activation/focus target.
   Support ordinary layout, text/styled text, images and icons as composed content;
   explicitly audit passive animated/loading/avatar/highlight content. Do not add
   nested editors, links, buttons, scroll owners or synchronous callbacks under a
   single semantic link. Specify permitted wrappers, resource leases, selectable
   text suppression, pointer occlusion and child AX exposure. Existing arbitrary
   `AnyElement` upstream composition does not justify an unsafe unrestricted FFI
   subtree. More complex interactive composites keep independent controls.
3. **Native admission and rendering.** Add a dedicated Link node/config operation
   and capability only once the complete path works. Validate kind/config pairing,
   passive descendants and handler eligibility on the final transactional tree,
   including dirty ancestors after Bind/SetStyle/SetImage/SetAccessibility updates.
   Invalid batches roll back atomically. Preserve all existing button icon rules.
   Retain the root's native focus state while updating content/configuration.
4. **Focus and input.** Reuse the native action/focus gate rather than forwarding
   raw OCaml key callbacks. Pointer, Return, Space and AXPress each enqueue once;
   disabled, hidden, inert, stale-generation and modal-ineligible roots enqueue
   nothing. Focus requests do not activate. Tab opt-out remains explicitly
   focusable when eligible. Index changes preserve current focus.
5. **Ordering across focus scopes.** GPUI's tab map orders index paths, whereas
   GPUIO's current modal/inert fallback uses recorded paint order. Merely calling
   `FocusHandle.tab_index` would therefore be incomplete. Define and implement
   consistent ordering for ordinary and gated traversal, preserving paint order
   for ties and existing controls at index zero. Verify mixed controls and native
   compound widgets, Tab/Shift-Tab, modal traps/restoration, hidden/inert branches
   and dynamic order changes without temporarily focusing an ineligible control.
   Native group/path interactions require implementation review and tests before
   freezing the scope semantics in public docs.
6. **Meaningful evidence.** Independent operation fixtures, malformed transaction
   rollback/bounds, OCaml reconciliation/current callback/stale delivery tests,
   native GPU/AX/input/resource tests, and a public gallery card. Use real pointer
   and keyboard input locally; distinguish AX automation from VoiceOver. Test
   content replacement/removal, theme/style changes, disabled/re-enabled roots,
   image lease disposal and idle stability. Then update both link catalog rows.
7. **Release integration.** Full local checks, required macOS/Linux builds/unit/
   private-bus/consumer CI, fresh installed gallery, reviewed PR and final release
   acceptance remain separate gates. Linux desktop qualification stays OCH-47.

This closes neither the link review nor milestone 07 by itself. Other catalog
families, macOS IME/VoiceOver, predeclared performance/resource workloads,
clean-machine distribution and release documentation remain required.
