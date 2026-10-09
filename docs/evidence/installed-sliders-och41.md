# Installed slider input and paint — OCH-41

On 2026-10-05, the independently installed public gallery passes an expanded
slider walkthrough on macOS 14.5 arm64 / M1 Max / built-in display. Application
source is unchanged at `e210d3e4a7b261e6bdf40023023937fbeb096e85`, executable SHA-256
`5e96404a5df658b7dfa9af05b8d4ff8240737e8df0f751f05c58968a0e24d495`.
See [the installed-consumer build](gallery-lifetime-repairs-och41.md).
Only the Python driver and documentation change at this checkpoint.

## Actual desktop coverage

- Range thumbs have separate native focus stops. Real Right/Left and
  Tab/Shift-Tab change and traverse the intended thumb. Ordinary AX numeric values
  and bounds follow the current opposite thumb; setting the lower thumb past the
  upper stops at the upper value.
- Real Home, End, Page Down and Page Up obey the linear domain. Horizontal pointer
  dragging previews 75 while committed remains 50, then commits 75 on release.
  A subsequent preview of 25 is cancelled by Escape, restoring committed 75.
- Read-only controls retain focus and remain enabled while rejecting keyboard and
  AX value mutation. Disabled controls report disabled and reject AX mutation.
  Re-enabling retains all three thumb identities.
- Axis and logarithmic-domain updates retain owners. Vertical logarithmic End
  reaches 1,000; Page Down reaches 990. Pointer dragging previews and commits 10;
  a later preview of 100 cancels back to 10. Replacing the native value with 1,000
  and restoring the linear domain clamps to 100. Values and labels reflect native
  observations through the public Eio controller.
- Thirty-two physical GPU cases cover Light/Dark × horizontal/vertical ×
  small/large targets × selected/remaining fill × single/range. Actual window
  pixels at three rail positions distinguish the filled and unfilled segments;
  thumb-center pixels check the independent thumb color. Remaining fill changes
  the single slider only; range fill stays between its thumbs. Target bounds are
  20 or 40 logical points, and AX identities survive all appearance updates.
- Resetting presentation settings preserves owners. Page departure removes the
  sliders; remount creates new native identities and restores mount-only initial
  values (35 and 20–80). The new single owner accepts Right to reach 36. The driver
  closes and reaps the application normally.

The Dark vertical/large/remaining capture was visually inspected: fill runs above
the single thumb, the range fills only between its thumbs, and the independent
thumb color and focus outline are visible. Pixel checks allow six RGB channel
values for color conversion and use bounded presentation retries. They do not
measure every corner radius, spring trajectory or ring frame.

## Harness corrections and preserved failures

Run 1 fails before slider input: the shared reveal helper demands a top margin
that the first setting cannot reach when the page is already scrolled to the top.
Settings now use their ordinary AX actions, including offscreen configuration;
actual sliders are separately revealed for physical input and paint. Run 2 then
passes the original walkthrough and all 32 paint cases.

Run 3 adds vertical logarithmic pointer cases. Committing 10 passes, but aiming at
100 reports 99. The native slider deliberately preserves the initial grab offset;
the test calculated an absolute endpoint while assuming AX's reported thumb
center was exact. The corrected test translates the actual grab point by the
requested change in logarithmic fraction. Run 4 passes exact 10 and 100 assertions,
commit/cancel observations, all prior input checks and all 32 paint cases. No
production behavior, value tolerance or test expectation was changed to accept 99.

## Reproduction and limits

```sh
python3 scripts/test_gallery.py --section sliders \
  --executable <installed-main.exe> --images <output>
```

The focused selector is also wired into `all`; only the focused walkthrough ran
at this checkpoint. The [archive](installed-sliders-och41/local-validation.tar.gz)
and [manifest](installed-sliders-och41/manifest.json) retain exact commands, all
four exit results/logs and driver versions, failure captures, intermediate sample
data and final captures/samples. Python compilation, catalog audit and whitespace
checks pass.

These checks do not qualify physical hover-spring timing/reduced-motion changes,
every controller-command button, full application resource/performance, the full
gallery, final-source hosted CI or distribution. Existing deterministic native
spring and command checks remain separately scoped in [the earlier evidence](slider-presentation-och41.md).
VoiceOver remains on owner hold and was untouched; ordinary AX inspection is not
screen-reader acceptance. Linux desktop qualification remains deferred to OCH-47.
