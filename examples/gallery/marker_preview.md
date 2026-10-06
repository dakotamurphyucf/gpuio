# Typed marker content walkthrough

Read [marker_preview.ml](marker_preview.ml) and its [interface](marker_preview.mli).
[pages.ml](pages.ml) mounts it on **Presentation**. `B` is `Bonsai.Cont`, `V`
`Gpuio_bonsai.View` and `M` Presentation.Marker. Graph hosts reactive options; `let%arr` reads
current state/palette to derive typed composition.

State machines cycle Plain/Separator/Border, Spinner/Shimmer, and no/custom/empty icon; another
counts actions. `B.state_machine0` returns current model plus injector reducing against latest
state when the effect executes. Toggles start typed/rich true and busy/empty/refined/compact
false. Native Marker variant activation runs its unit-action effect, updates variant, derives
`M.create` and reconciles native composition. Constructing effects does not start work or change
model.

[`M.Content.Item.text`](../../lib/core/presentation.mli) provides validated selectable “Thinking
· 京都” or empty typed text. `Item.element` inserts the ordinary keyed action button.
`Content.create` validates unique keys and remains a stable native animation root across
loading/variant changes. An arbitrary root Steady element stays outside it. Typed M.Icon is a
centered slot; even empty icon explicitly suppresses the automatic spinner. No SVG/asset
registration is needed for the diamond text.

Loading Spinner adds a meaningfully labelled native indicator if no typed icon. Shimmer affects
only typed text; content without typed text pulses its styled opacity, while mixed rich
children/root elements remain unchanged. Empty typed text still counts as text and suppresses
rich-only pulse. Reduced motion restores ordinary native paint; no frame callback reaches OCaml.
Content consumes an advanced animation owner even static, bounded to 1,024 owners
application-wide; managed histories need bounded mounting.

Set busy, switch Shimmer, remove typed text, then compare pulse; restore empty typed text to
test precedence. Click Marker action while busy: its effect increments the latest counter and
derives action readout; busy does not disable this child. Removing rich content removes that
action owner. Root named Group adds no live announcement; activity label does not announce
automatically.

Bonsai owns preferences/counter, GPUIO native animation/input. No Eio tasks/assets/cleanup
callbacks exist. Adapt with stable nonreserved keys, validated UTF-8 text at most 16,384 bytes,
explicit child availability and meaningful status semantics. Marker animation describes supplied
state; it starts no job and supplies no work cancellation/persistence.

## Run and review

From the repository root:

```sh./scripts/gpuio build examples/gallery/main.exe./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not checks run for this documentation
change. There is no separate executable/self-test for this component. Compilation does not
establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
