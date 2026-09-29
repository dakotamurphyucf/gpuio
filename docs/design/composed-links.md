# Composed links and native focus policy

OCH-41/OCH-17; pinned-source review at GPUIO `1785139`.
Status: **configuration foundation implemented; mounting, native validation and
public gallery still required**. This document does not claim link-family parity.
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

No native opcode, node kind or capability is added by this configuration phase.
The wire definition is a development foundation on the shared release branch,
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

## Remaining implementation contract

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
