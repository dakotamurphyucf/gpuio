# Rich Bubble and Message composition

OCH-41 adds `Presentation.Bubble` and `Presentation.Message` alongside the original
single-view/string helpers. The reference is gpui-kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`, captured in
[Bubble](../catalog/sources/component-bubble.rs.txt) and
[Message](../catalog/sources/component-message.rs.txt). These two source rows are
locally validated functional equivalents on macOS.
The contracts and scoped evidence below do not complete the presentation family
or the broader release gates.

```ocaml
let module P = Gpuio.Presentation in
let bubble =
  P.Bubble.create appearance ~variant:Ghost
    [ Gpuio.View.text "A place for the next idea." ]
in
let content =
  P.Message.Content.create
    [ P.Message.Content.Item.bubble
        ~key:(Gpuio.Key.of_string_exn "response")
        bubble
    ]
  |> Core.Or_error.ok_exn
in
P.Message.create appearance
  ~header:(P.Message.Header.create [ Gpuio.View.text "Assistant" ])
  ~content
  ~footer:(P.Message.Footer.create [ Gpuio.View.text "Delivered" ])
  ()
```

A Bubble is a stateless typed descriptor: `Bubble.view` returns its ordinary view,
and `Bubble.variant` returns the authoritative variant. `Message.Content.Item.bubble`
retains that metadata, allowing message header/footer defaults to adapt to Ghost
surfaces. `Item.element` accepts any view but does not infer a variant from colors,
styles or descendants. Explicit Ghost with custom padding is still Ghost for this
policy, matching the source's variant-based decision.

## Bubble contract

The root is a relative, non-growing/non-shrinking column with minimum width zero,
4px gap and maximum width 80%. Ghost changes width/maximum to 100%. Optional shared
`Presentation.Alignment.Start`/`End` sets self alignment and an opposite auto margin;
omission leaves the parent in charge. A message never overwrites explicit bubble
alignment. Root `style` refines these defaults.

Alignment applies separately to the content surface too. An explicitly aligned
auto-width surface can shrink inside a wider root. Reactions anchor to the root's
layout box, not the visible surface. These are distinct boxes even when their
default stretched geometry happens to coincide.

A separately keyed content surface has minimum width zero, maximum width 100%,
clipped overflow, 16px radius, one-pixel transparent border, 12px horizontal/8px
vertical padding, font 14px and line height 162.5%. `content_style` refines the
surface independently from root placement; ordinary supplied children retain
their own explicit styles. Children form a vertical composition.

| Variant | GPUIO default surface |
| --- | --- |
| Filled | Accent background, on_solid foreground |
| Secondary, Muted | Raised background, ordinary foreground |
| Tinted | Accent at 12% of its original alpha, ordinary foreground |
| Outline | Surface background, ordinary foreground, border color |
| Ghost | Transparent background, ordinary foreground; zero radius, border and padding |
| Destructive | Danger at 10% of its original alpha, danger foreground |

These are explicit Appearance-based defaults. They preserve theme tokens/alpha
but do not copy the source's semantic theme registry, dark/light Oklab mixing or
exact radius values. Applications can refine every surface with styles.
`Bubble.group` supplies a minimum-width-zero column with 8px gap.

## Reactions and measured layout

`Bubble.Reactions.create` accepts uniquely keyed items, optional style, Top/Bottom
(default Bottom) and Start/End (default End). The wrapper is an absolute row with
4px gap, full radius, 3px surface-colored border, raised fill and ordinary foreground.
It attaches at Top -20px or Bottom -20px, with a 12px horizontal inset. Styles refine
those defaults. Moving the wrapper changes styles, not the surviving control keys.

`Reactions.Item.action` creates an ordinary native button, forwarding the same
name/disabled/icon/style and asynchronous callback contract as `View.button`, with
a 999px pill radius applied last. A typed action suppresses the wrapper's default
6px horizontal/2px vertical decorative padding. An arbitrary `Item.element`, even
a button, keeps its original styles and does not suppress padding. This distinction
matches the source and keeps a predictable escape hatch for custom controls.

**Absolute reactions do not enlarge measured row height.** Callers must reserve
margin, group gaps or row spacing when reactions should not overlap neighbors.
A managed list measures the ordinary layout box, not out-of-flow painted controls.
Ancestor overflow clipping still applies to both paint and pointer hit areas.
The gallery reserves top space on the bubble and a larger message/footer gap;
these are example layout choices, not hidden changes to the source defaults.

## Message slots and inheritance

`Message.create` accepts optional Avatar, Header, Content and Footer descriptors,
root/stack styles, and Start/End alignment (default Start). Named native parents
stay stable while other slots come and go. The root is full-width, minimum-width-
zero, with 10px gaps, font 14px and 125% line height.

The body row has an 8px gap and bottom-aligns its avatar with the header/content
stack. End reverses this row. Avatar defaults to minimum width 32px, no grow/shrink,
centered children, full radius, clipped overflow and a raised surface; its height
comes from content unless styled explicitly. The stack is full-width/minimum-width-
zero, gap 10px, with independently refined styles.

Footer lives **outside** that row: growing the footer does not change the body
row's bottom edge. Its default avatar-column margin is 40px (32px baseline + 8px
gap), on the matching side. Footer style applies last and can override the margin
for custom avatar sizes; there is no synchronous measurement callback.

Header/Footer are rows with maximum width 100%, minimum width zero, 4px gap,
font 12px, weight 500, line height 125% and muted foreground. `content_inset=None`
uses 12px horizontal padding unless Content contains any typed Ghost bubble.
Explicit true/false overrides that inheritance. Styles refine the resulting slot.

Content is a full-width column with 10px gap. Its unique keyed items are typed
bubbles or arbitrary ordinary views. Message alignment positions these children;
it does not rewrite a Bubble's own alignment. Duplicate content/reaction keys are
rejected before reconciliation. `Message.group` uses an ordinary 8px-gap column.

## Ownership and accessibility

The compositions own no tasks, resources, text models or clocks. Editors, registered
Markdown/code sources, Eio producers and asset scopes stay application-owned.
Pure descriptions preserve native identity through stable keys and parents; slot
removal retires that subtree and ordinary late-event fencing applies. Application
model retention across remount is separate from native editor/resource lifetime.

No automatic role, live region or extra focus stop is introduced. Text, documents
and interactive children retain their ordinary native semantics/actions. Applications
can apply accessibility metadata to ordinary views explicitly. Background geometry
or AX inspection is not VoiceOver, keyboard focus or IME acceptance.

`Bubble.with_accessibility` validates root metadata through the ordinary View
contract while preserving the typed variant, keys, styles and children. This lets
applications label a bubble without losing Message's Ghost inset policy. An
incompatible role (such as Link on this non-interactive container) is rejected.

## Local acceptance and remaining release gates

Five Core expect checks pass: root metadata validation/preserved Ghost policy,
duplicate reaction/content keys, typed-action versus
arbitrary style/padding, seven surfaces, Ghost width and root/content refinement,
typed Ghost/inset decisions, 32 optional-slot/alignment combinations, and 28 streaming/
variant/alignment/reaction-side transitions. Body/reaction controls retain native
IDs and current callbacks; equal snapshots are idle and retired actions are rejected.

The public Presentation card **Room for a conversation** contains a real native
editor, scoped streaming Markdown/code source, all variants and slots, independent
alignment/inset choices, typed/arbitrary reaction controls, clipping and narrow width.
The document source is bounded to six appended chunks and retired on page departure.
The source counter resets with its scope; unrelated Bonsai action counters remain.

The corrected repository-native run passed 28 theme/variant/alignment geometry cases,
56 OS Return/Space actions, 16 reaction pointer actions outside the bubble surface,
eight clipped attempts correctly suppressed, typed/arbitrary padding and identity,
footer growth, compact width and Ghost/inset policy. The editor retained its typed
value and focus through three Markdown/code appends. Independent Bubble alignment,
slot retirement, page remount and zero registered source bytes after departure pass.
Fourteen GPU surface cases cover seven variants in both themes, including Outline
border paint. Earlier harness errors used AXTextArea for Markdown's AXGroup and
conflated an explicitly aligned surface with its wider root; both are corrected.

The separate **A conversation in motion** preview uses 100 keyed messages with a
12-row bound, a 360px viewport and an independent native draft. A scoped document
belongs to the preview, survives row eviction and accepts at most twelve chunks.
Each row reserves 28px below its message for absolute reactions. Its focused check
has passed monotonic streamed growth, stationary draft, paused-history geometry
during offscreen appends and an outside-surface reaction action. Small-scroll
validation exposed a real routing defect: one pixel moved both the list and its
ordinary ancestor. A failing-before native regression and the repaired native/public
walkthroughs
cover the [routing correction](../evidence/scrolling-och11.md#milestone-07-managed-list-inside-an-ordinary-scroller).
No GPUI fork or protocol change was needed.

The final fresh installed-library consumer passes both focused sections. The
composition matrix repeats all 28 layout cases, fourteen GPU surfaces, 72 body/
reaction actions, editor value/focus retention, streaming and slot/page teardown.
The final managed transcript run records 85 retained-AX samples with monotonic preceding-row
movement and a stationary draft; three further appends while the final row is
unmounted preserve the history anchor. The test explicitly checks document absence
and a nonempty active set of at most twelve rows. Sixteen one-pixel forward/reverse OS wheel
events keep the outer viewport fixed and the warm reaction control alive. The
outside-surface reaction fits inside reserved row space and activates; jumping to
latest restores the current document. Page departure releases all source bytes,
and remount creates a fresh native draft owner. The repository run also passes
with 82 stream samples. These bounded observations are not frame-time budgets.

Commands used on macOS 14.5 arm64, 2026-09-30:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @fmt
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-m7-chat-consumer2-20260930
python3 scripts/test_gallery.py --section chat-composition --executable /private/tmp/gpuio-m7-chat-consumer2-20260930/consumer/_build/default/main.exe
python3 scripts/test_gallery.py --section chat-list --executable /private/tmp/gpuio-m7-chat-consumer2-20260930/consumer/_build/default/main.exe
```

Both gallery runs used 480-second process-group watchdogs and exited zero; every
owned child was reaped. The consumer stages libraries in its own prefix without
changing an opam switch and builds its own locked native backend. Full Dune passed
for the composition API/previews; the subsequent native routing repair also has a
passing gallery rebuild/format check, strict Clippy, 413 native unit tests and the
full native scroll/list regressions (including two 100,000-row traversals,
selection/editor retention and warm-row/resource bounds).

No combined-gallery, hosted CI, Linux GUI, VoiceOver, real IME or application-
performance acceptance is claimed here. OCH-17 and required non-GUI Linux release
gates remain; full Linux desktop qualification is deferred OCH-47. The pure
composition contracts add no native capability bit or additional resource owner.
