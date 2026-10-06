# Message follow: overlays around one scroll owner

[message_follow.ml](message_follow.ml) and its [interface](message_follow.mli)
provide a pure GPUIO view recipe for a bottom fade and Follow latest button.
Unlike a Bonsai component, this function consumes concrete current inputs and
returns a view; it allocates no reactive state, list controller, animation clock,
Eio task or scroll owner. The supplied managed list remains the only scroll owner.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Lists, trees & tables → Message list**. Scroll away from the tail, toggle
follow/fade/motion controls and click Follow latest. There is no helper-specific
entry point, diagnostic flag or external asset. [Development](../../../docs/development.md)
explains toolchain setup; the [gallery README](../README.md) records platform limits.
This review adds no scrolling, animation, physical input or VoiceOver acceptance.

Read `is_away`, the local animation/gradient helpers in `view`, then the wrapper's
three children. `is_away` returns false before a viewport observation exists.
For Some viewport it returns true only when **both** following_tail=false and
at_end=false. A short list at its end does not suggest unread history even if it
isn't following; following tail also suppresses the overlay. The caller must supply
its current accepted [Virtual_list.Viewport](../../../lib/core/virtual_list.mli),
not infer this policy from an arbitrary scroll offset.

`view ~viewport ~jump_enabled ~motion ~fade ~jump list` takes ordinary values.
Its type is polymorphic in the view action, so the recipe can carry a Bonsai effect
button without itself knowing Bonsai. The caller supplies `jump`, already bound to
the current managed-list controller and styled to fit a 48-pixel slot. `jump_visible`
is jump_enabled && away; the fade can appear while away independently of that button.
Omit fade to disable the gradient.

## Layout, hit testing and native animation

The root key is `message-follow`, with Relative positioning and hidden overflow.
Its first child is the unchanged list. `bottom-fade` is an absolute 48-pixel overlay
at the bottom, inset 18 pixels from the right edge, with Pointer_events=false.
Its 180-degree gradient runs from transparent to the supplied fade color; when
fade=None the retained overlay is transparent, not removed/recreated.

`jump-motion` is another absolute 48-pixel wrapper, also noninteractive itself.
Native animation targets opacity 1/0 and bottom 16/-48 depending on jump visibility.
The jump button lives inside `jump-gate`, with Pointer_events=true and
`Inert (not jump_visible)`. Availability therefore changes immediately, even while
native wrapper opacity finishes fading. Hidden targets settle below the viewport
and cannot shield message pointer input. Absolute overlays do not change list
measurement, scroll layout or row budget.

Both animation configs use validated `Animation.Target`, Ease_out and duration
200 ms when motion=true, zero otherwise. Native reduced-motion policy still applies.
The helper sends targets; it neither computes animation frames in OCaml nor reports
that an opaque view has already painted. Contracts are in
[animation.mli](../../../lib/core/animation.mli) and
[style.mli](../../../lib/core/style.mli).

## Follow a viewport event back to the tail

[collections_page.ml](../collections_page.ml) constructs a managed list with
Follow_tail_when_at_end, then reads `L.Output.viewport list` in its outer `let%arr`.
Bonsai reactive values hold jump/fade/motion preferences, but this helper receives
their concrete current values. A native scroll observation updates the managed
list's reactive output; scrolling away makes is_away true, and Bonsai derives this
wrapper with visible follow targets. This does not replace or reconfigure the list.

The supplied button runs `L.Controller.jump_to_latest (L.Output.controller list)`.
That generation-bound effect asks the native list to follow tail. A subsequent
viewport observation reports following_tail/at_end, the caller derives is_away=false,
and the recipe makes the button inert immediately while fading the wrapper out.
Native scrolling/measurement and animation are independent of row data updates.
Lifecycle, stale scroll commands and native cleanup belong to the managed-list
controller, not these pure wrapper values.

The [existing expect tests](../../../test/gallery/message_follow_test.ml) enumerate
unknown/following/end combinations and inspect inert/animation targets. They also
reconcile presentation changes and assert no list create/remove/order/rows/config/
scroll operations are emitted. These are existing pure/protocol test cases, not new
GUI or frame evidence. No tests were executed for this guide.

To make the button rise 24 pixels, change its visible Bottom target while retaining
the hidden -48 target, 48-pixel clipping slot and immediate inert gate. To style the
fade, pass a concrete resolved background color; do not add a second scroller or
move overlays into normal layout flow. Preserve stable wrapper keys and provide the
current generation-bound jump action instead of retaining an obsolete controller.
