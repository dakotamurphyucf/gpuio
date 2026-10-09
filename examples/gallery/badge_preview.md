# Badge preview walkthrough

Read [badge_preview.ml](badge_preview.ml) and its [interface](badge_preview.mli). [pages.ml](pages.ml) mounts `component app window palette graph` on **Presentation**. Run the gallery using the [development commands](../../docs/development.md); this preview has no separate executable or self-test.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

The [gallery README](README.md) records prerequisites and platform limits.
This source walkthrough adds no native input, VoiceOver or platform acceptance.

`kind` is a local sum type: `Count of int`, `Dot`, `Icon` or `Decorative_icon`. `B.state` starts with `Count 7` and `Presentation.Size.Medium`; `B.toggle` starts the cap at 99. A `B.state_machine0` increments the mock inbox-open count. These allocate state in the Bonsai graph. `let%arr` reads their current values and the palette, then constructs a new view description; effects such as `set_kind value` run only when a button activates.

The [scope helper](preview_scope.md) registers the inline check SVG through `Gpuio_eio.Asset.register`. `Effect.map` converts the successful registration to a borrowed handle and preserves errors for the loading/error branches. Registration happens even when the selected badge is a count. The visit owns the registration; leaving the page cancels pending acquisition and disposes completed resources, while returning acquires fresh resources. GPUIO owns native painting and asset use; this component has no network inbox or polling task.

In the ready branch, [`Presentation.Overlay_badge`](../../lib/core/presentation.mli) validates count/dot descriptions. Counts and caps must be nonnegative. Zero hides the overlay; values above the cap display `max+`, while the accessible label still states the actual count. The dot has a meaningful text alternative. `Icon.Config.create` borrows the SVG with either the label “Verified inbox” or an explicitly decorative description. Both icon variants use the success tone, while count and dot use danger and accent.

`Presentation.overlay_badge` wraps the ordinary “Open inbox” button. Count/dot overlays sit at the top right and icons at the bottom right. `size` changes the badge, not the button; the wrapper is 170 pixels wide. The overlay is pointer-passive and has no independent action or focus target. Its semantic label does not itself request a live announcement. The surrounding group is named “Badged inbox”.

Select **Unread: 150**, then switch **Badge cap: 99** to 9: the visible badge changes from `99+` to `9+` without changing its actual-count label. Select **Unread: 0** and activate **Open inbox**: the overlay disappears but the content button still increments the count. Switch between verified and decorative icons to compare semantic descriptions. These display changes preserve the content button’s native identity.

To adapt this example, supply real application count state and an actual open-inbox effect. Keep the uncapped semantic count and choose decorative icons only when surrounding content already communicates their meaning. Handle validation errors explicitly for external data; `ok_exn` here assumes bounded, trusted constants. Keep asset acquisition in a visit scope rather than registering a new icon during every `let%arr` evaluation.
