# Aspect preview: preferred proportions without replacing child state

[aspect_preview.ml](aspect_preview.ml) is a small Bonsai component called from
[styles_page.ml](styles_page.ml). There is no separate interface. It demonstrates
`Style.Property.Aspect_ratio` while a nested counter remains application-owned.
No asset, registration, Eio producer, I/O or native editor is involved.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Styling details**, find **Proportions that follow your layout**, increment
Kept and change ratio/frame/height. There is no dedicated self-test or diagnostic
flag. [Development](../../docs/development.md) gives toolchain prerequisites; this
documentation review establishes no physical layout, input or platform acceptance.

`component` stores ratio index 0 in `B.state_machine0`, cycling modulo three through
Square=1, Landscape=2 and Portrait=0.5. Two `B.toggle` values start false for wide
frame and fixed height. Another state machine owns count zero and its injection
effect. State is constructed once in the graph; `let%arr` combines current palette
and state into a view. Native button callbacks schedule effects, not reinitialize
these state machines during layout.

The outer frame keeps key aspect-frame and width 140 or 200 logical pixels with
12-pixel padding/border. The child keeps key aspect-preview, width 100%, the chosen
aspect ratio and Shrink=0. Automatic height lets the ratio influence height; fixed
height supplies 80 logical pixels explicitly instead. Ratio is a preferred layout
constraint, not an instruction to overwrite both definite dimensions. Centering
places the Kept button within the surface. Local `label` wraps frame/surface with
named accessibility Group metadata; it creates no additional action.

Click Kept to reach one, then cycle Landscape and widen frame. The injection effect
changes count or ratio/width; `let%arr` derives new styles with the same keys and
counter graph. Native layout updates the surface while the count remains one.
There is no OCaml resize loop and no background task to cancel. Read
[style.mli](../../lib/core/style.mli) for typed style/length properties. To add a
16:9 ratio, extend the ratio array and matching modulo bound together; use finite
positive ratios and retain child keys/state placement. Do not move counter state
into newly selected ratio branches if you intend it to survive ratio changes.
