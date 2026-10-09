# Installed split paint and menu placement — OCH-41

On 2026-10-05, the independently installed gallery passes two additional physical
macOS walkthroughs: split-button GPU state painting and menu anchor placement.
The application is unchanged from `e210d3e4a7b261e6bdf40023023937fbeb096e85`,
executable SHA-256 `5e96404a5df658b7dfa9af05b8d4ff8240737e8df0f751f05c58968a0e24d495`.
See [the consumer build and lifetime repairs](gallery-lifetime-repairs-och41.md).
Environment: macOS 14.5 arm64, M1 Max, built-in display. Only test drivers and
documentation change at this checkpoint. VoiceOver remains on owner hold and
was not tested, automated or configured.

## Split painting

`gallery_split_paint.py` checks 20 cases: each of the following in Light and Dark.
It captures the actual owned window, decodes the GPU pixels with system ImageIO,
and checks a 3×3 interior patch on each half against the example's palette colors
(at most six channel values of tolerance). It retries a bounded number of times
for physical presentation; every attempted capture is retained in the focused
run's evidence. Geometry checks require equal heights and no gap at the join.

| State | Primary surface | Menu-trigger surface |
| --- | --- | --- |
| Rest; pointer in blank space beyond the pair | Card surface | Card surface |
| Hover primary | Accent | Shared surface |
| Hover trigger | Shared surface | Accent |
| Return to blank space | Card surface | Card surface |
| Menu open, pointer outside both parts | Shared surface | Accent |
| Escape closes menu | Card surface | Card surface |
| Primary disabled, hover trigger | Card surface | Accent |
| Primary loading, hover trigger | Card surface | Accent |
| Pair disabled, pointer over trigger | Card surface | Card surface |
| Pair recovered, hover trigger | Shared surface | Accent |

The first focused run exits 0 with both success markers. Representative Light
menu-held and Dark trigger-hover captures were visually inspected: the joined
inner edge and rounded outer corners are visible, and only the intended half
uses the accent. This does not measure corner radii or border thickness across
all scales. Page departure removes both controls and the menu.

## Menu placement and root scrolling

`gallery_menu_placement.py` checks Bottom/Right × Start/End × Light/Dark: eight
openings and eight additional measurements after actual root wheel scrolling.
The driver centers the trigger to avoid a viewport clamp when checking the
preferred side. It checks the configured eight-point gap and cross-axis alignment
against the live `AXMenu` and trigger bounds, within two logical points for panel
borders and rounding. All 16 measured origins differ from the expected origins
by at most one logical point. Each wheel sequence moves the trigger 43 points;
the popup follows while the observed root remains open. Trigger identity is
retained. Escape closes the menu and returns focus to that trigger.

The first focused run exits 0 with both success markers. The Light Right/End
capture was visually inspected against the reported alignment. The default menu
appearance follows the native window appearance; changing the gallery's explicit
palette does not itself override that native default. This is geometry evidence,
not a claim of identical popup colors under the two gallery palettes.

The existing production-View matrix separately covers all four sides, three
alignments, scales 1/1.5/2 and edge flips; see [the design checkpoint](../design/menu-observation.md).
Those tests use TestPlatform, not physical desktop input. This new physical
walkthrough covers the public example's two sides and two alignments, plus root
scrolling; it does not replace or broaden the other matrix's platform scope.

## Reproduction and remaining scope

```sh
python3 scripts/test_gallery.py --section split-paint --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section menu-placement --executable <installed-main.exe> --images <output>
python3 scripts/test_gallery.py --section buttons --executable <installed-main.exe> --images <output>
```

Both focused selectors are also included in `buttons` and `all`.
The combined `buttons` run also exits 0: rich buttons, appearance/Link,
menu observations, all 16 placement samples, split modes/identity, all 20 paint
cases and live command hints finish in the same application process. It closes
and reaps normally. This checks the new scenarios after prior state changes and
page remounts; `all` was not run at this checkpoint. Python compilation, catalog
audit and whitespace checks pass.
The [archive](installed-split-paint-menu-placement-och41/local-validation.tar.gz)
and [manifest](installed-split-paint-menu-placement-och41/manifest.json) preserve
commands, process exit results, driver snapshots, captures and per-case samples.
These scoped results do not establish VoiceOver, Linux desktop, full-family
resource/performance, the full gallery, final-source hosted CI or distribution
acceptance. OCH-41 and OCH-17 remain in progress; Linux desktop remains OCH-47.
