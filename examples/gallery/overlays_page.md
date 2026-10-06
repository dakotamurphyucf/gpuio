# Overlay state and native focus walkthrough

Read [overlays_page.ml](overlays_page.ml) and its [interface](overlays_page.mli).
[pages.ml](pages.ml) routes Overlays to `component palette graph`. `B` is `Bonsai.Cont`, `V`
`Gpuio_bonsai.View` and `E` `Bonsai.Effect`; graph hosts reactive state, and `let%arr` derives
views from current values.

`Modal.t` is Closed/Dialog/Sheet/Confirm, so only one of those modal contents is supplied at
once. A separate boolean owns the anchored popover. Tinted backdrop/animation start true,
reserved chrome false and sheet edge Right; notice starts unchanged. Constructing setter effects
while building buttons changes nothing. Native Open dialog activation executes
`set_modal Dialog`, updates that model, and `let%arr`/`content` derive Some dialog content for
GPUIO to mount with native modal focus policy.

`content which f` constructs children only for the current modal; `panel` supplies ordinary
content and `overlay` creates width 440 with outside dismissal enabled. Native dismissal
requests run `close`, updating modal to Closed, deriving absent content and removing it. The
[Overlay contract](../../lib/core/overlay.mli) keeps focus policy active until that removal is
accepted; requesting dismissal is not already closing the native modal. Entry animation is
native, reduced motion settles, and removal is immediate rather than an exit animation.

Popover Show details/Done uses its separate boolean and anchor. Three tooltips have
native-managed transient visibility and optional Enter_and_switch motion; “Streaming help” is
fixed text, with no streaming source. Hover card includes an interactive profile action that
only sets notice. The [placement preview](placement_preview.md) is an independently composed
child.

Sheet.Config uses extent 380, one of four attached edges, and optional top/right/bottom/left
insets 56/16/16/16. Moving edge updates presentation of the same active sheet; reserved space
remains covered by modal backdrop. Confirm reset executes `E.Many` to update notice and close;
it changes no files. Transparent backdrop changes paint, not whether modal focus policy applies.

`B.Edge.lifecycle` closes modal/popover on page deactivation, preventing requested opening from
lingering into the next visit. GPUIO owns overlay stack, focus containment/restoration,
placement and motion; Bonsai owns requested content/state. There is no editor, asset scope or
background Eio work in this module. Adapt by tying real confirmation effects to results and
closing state explicitly, rather than treating a dismissal callback as automatic removal or a
visual backdrop as a focus policy. Keep independent nonmodal/modal states intentional if your
product permits their coexistence.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks run for
this documentation change. The page has no standalone executable or self-test. Compilation does
not establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
