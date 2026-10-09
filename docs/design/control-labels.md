# Rich checkable labels

OCH-41 implementation contract, 2026-10-01. In progress; this document alone is
not an implementation or native-acceptance claim.

## Public shape and semantics

Keep the existing string-label constructors. Add checked
`View.checkbox_with_label` and `View.switch_with_label` constructors with one
rich `View.t` label, a required `accessible_name`, and the existing state,
callback, disabled, root-style and appearance arguments. A nonblank UTF-8 name
without NUL, at most 1024 bytes, names the control independently of visible text.
Blank checking uses Core `String.strip` whitespace on both sides of the bridge.
Returning to the string constructor with the same key preserves the native
control; the removed label subtree releases its resources.

`View.radio_group_with_labels` takes the existing `Choice.Config` and a list of
`Choice.Id.t * View.t` overrides. Reject unknown or duplicate IDs. Omitted IDs
retain their ordinary string labels; option names and selection remain in the
validated Choice collection. Reordering options preserves label slots by option
ID, while removing an option retires its label. Changing a display string does
not change identity. Mirror these constructors in the Bonsai facade.

Labels are passive compositions: containers, plain/styled text, decorative
images/icons/avatars, loading and native animations. Reuse the existing passive
content contract: no callbacks, nested controls, selectable text, scroll regions
or pointer shields. Limit the whole label content to 4096 nodes and 128 levels.
Animated/image resources retain their ordinary scoped ownership and native
painting; hiding label semantics must not itself suspend their animation.

The checkable root (or each native radio option) owns activation and the accessible
name. Rich label descendants are hidden from the accessibility tree to avoid
announcing both the name and its decorative contents. Clicking visible label
content activates that owner once. Disabled/inert ancestors keep their existing
input policy. A control's disabled state (including a disabled radio option)
projects `Disabled` styles into its label, including delayed avatar fallback
content, without adding focus targets or repeatedly applying default dimming.
Root style controls layout, alignment, wrapping and focus styling;
`Control_appearance` still controls indicator size and Before/After label order.
Multi-line labels do not implicitly change indicator size or impose the styled
source's first-line alignment; callers can choose root alignment explicitly.

The pinned base components accept arbitrary Rust children. This API intentionally
models a single checkable target with passive labels. Independent links/actions
belong beside the control, not hidden inside its accessible label. This does not
claim arbitrary interactive-descendant parity. Standalone radio composition and
custom Tab order are connected through the [navigation extension](checkable-navigation.md);
actual desktop validation remains open.

## Native tree and protocol

Use existing Create/Splice/style/value operations, with capability bit 61
`CAP_CONTROL_LABELS` added only once the native path is connected. No new opcode.
Checkbox/switch roots allow zero children (legacy text) or one passive label root.
In rich mode the root's text carries the required accessible name and is not
painted as an additional label.

A radio group either has no children (legacy) or one container slot per configured
option in collection order. Each slot contains zero or one passive label root;
zero means the option's string label. OCaml keys these slots by option ID. The
final transaction must validate slot count, shape and passive descendants against
the final Choice configuration. Option reorder/configuration and child splices
are atomic. Both initial admission and dirty descendant updates must enforce the
same invariants. Plain tab bars/selects do not gain arbitrary children.

The slots and labels are ordinary retained nodes with normal quota accounting,
keyed reconciliation and unmount behavior. There is no synchronous OCaml callback
from layout or native accessibility. Hide label semantics through the semantic
wrapper while preserving layout, paint and normal resource lifecycle.

## Decorative semantic identity

The native label wrapper has its own stable element identity, namespaced with
both the label node's slot and generation. GPUI's type-erased `AnyElement` does
not forward an element ID; putting `hidden` metadata on an unidentified wrapper
would skip that ancestor during accessibility tree construction. The explicit
identity allows a hidden group to enclose the decorative subtree without adding
a layout box, action, focus target or clock. The native control remains the
single semantic owner. This applies to rich buttons and checkable labels alike.

A direct production-wrapper regression checks the type-erased case. Native
layout/input/resource tests also pass after the correction. These establish the
wrapper contract, not macOS AX traversal or VoiceOver acceptance; the physical
rich-label driver remains required.

## Required evidence

Core tests must cover checked names, forbidden descendants/styles/callbacks,
unknown/duplicate option IDs, partial overrides, reorder and fallback; theme/style
updates, plain/rich changes, current callbacks and label retirement must preserve
the owning control. Independent bytes establish the capability addition.
Native transactions must reject invalid child shapes and late descendant mutations
atomically. Production-View tests must establish one rendered label, both orders,
retained focus, per-option slot identity and cleanup. Actual GPU/AX/keyboard and
pointer label activation, animated/image lifetime and public repository/installed
consumer scenarios remain required separately from headless/build checks.

## Implementation checkpoint — 2026-10-01

The Core/Bonsai constructors, keyed slot reconciliation, bit-61 negotiation and
native admission/rendering are connected. The Controls gallery has a **Rich
control labels** toggle: multiline checkbox/switch descriptions and sparse radio
labels, with ordinary string fallback and the same owner keys in plain mode.
Four Core expect cases and four native admission/fixture tests cover names,
reconciliation, sparse/reordered radio overrides, late invalid mutations,
rollback and independent capability bytes. The full Rust workspace passes.
The native library passes 477 tests with two existing skips, and strict
native/protocol all-target Clippy passes. The full Dune test/format/gallery
build also passes.
A fresh independently installed gallery builds with `run=False`; it does not
establish native consumer runtime or clean-machine distribution acceptance.

A production-View TestPlatform regression covers all three control kinds: one
painted label in both orders, one simulated click routed to its owner, disabled
click rejection, inherited selection exclusion, retained focus, disabled styles
inside avatar fallbacks (with no image and with a failed primary), spinner
wakeups despite hidden label semantics, and zero retained bytes/idle after
removal. A second renderer check verifies radio reorder keeps the focus and
selection owner while moving each label and its disabled styling with the option.
These are headless checks, not OS input/accessibility evidence.

That regression exposed excessive debug stack use during recursive element
construction. Default widget presentation and checkable painting are now
separate helper stages, so their temporary builders do not accumulate at each
traversal level. The same nested regression passes with the default Rust test
stack; no stack-limit override was added. Maximum-depth release resource
qualification is still separate.

The public desktop appearance driver also includes rich-label geometry, hidden
decorative AX labels, caption pointer activation, keyboard, sparse radio fallback,
disabled/inert policy and rich-to-plain identity. Python compilation passes;
these new desktop assertions are unrun. Actual macOS GPU/AX/keyboard,
animated/image lifetime and repository/installed-consumer runtime acceptance
remain open.

Validation commands (repository-isolated macOS toolchain):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 --workspace
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib --test control_labels
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy --locked -j2 -p gpuio-native -p gpuio-protocol --all-targets --features gpuio-native/native-image-tests,gpuio-native/native-canvas-tests -- -D warnings
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/agents/root-20260929-m7-resumed/control-labels-installed-consumer
```

The workspace test uses no native GUI test features. The native-feature command
explicitly selects the library and transaction test, so it runs TestPlatform
checks without opening desktop windows. No current hosted CI or Linux graphical
acceptance is inferred from these local results.

## Decorative tab labels — 2026-10-02

`View.tab_bar_with_labels` and its Bonsai alias now share the checked Choice-ID
slot contract. The native TabBar permits and validates these slots, renders each
label once, and preserves its existing native tab names, selected/disabled state,
keyboard routing and retained focus. Ordinary `tab_bar` remains compatible; empty
label overrides restore string labels without replacing the tab owner. No opcode
or additional capability is introduced within the existing paired protocol.

This covers decorative rich labels only. Interactive prefix/suffix controls,
variant/per-tab target styles, overflow, explicit reveal and indicator motion are
separate required [rich-tab work](rich-tabs.md). See [evidence](../evidence/tab-labels-och41.md).
