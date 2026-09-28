# Component Studio implementation evidence

Status: partial OCH-41 implementation, 2026-09-28. The gallery and catalog audit
are not complete. This record is narrower than milestone 07 release acceptance.
Local platform: macOS 14.5 (23F79), arm64. Use the pinned repository environment.

## Implemented preview sections

| Section | Native/public behavior exercised | Remaining coverage examples |
| --- | --- | --- |
| Presentation | Public compositions, avatar fallback/image semantics, loading controls; screenshot inspected | Image success/failure and complete theme/scale matrix |
| Selection & actions | Button event into Bonsai; native checkbox/switch/radio/select/combobox previews | Full combination of disabled, keyboard and style states in gallery |
| Text editing | OS typing and Enter submit; form validation metadata, theme and preview-size changes retain native text | Real IME/clipboard acceptance remains in release audit |
| Numbers & codes | Single/range sliders, number stepper, OTP, rating; native slider AX increment reaches observed value | Gallery OTP/stepper/rating OS interaction and all scales |
| Dates & colors | Calendar/range and inline color previews; popup date/color changes cancel or confirm correctly; Escape and focus restoration | All constraints/disabled/read-only/scale combinations |
| Overlays & help | Dialog, drawer, confirmation, popover, tooltip and hover-card previews; native dismiss/confirm/focus checks | Tooltip/hover-card keyboard/hover and complete style/scale matrix |
| Navigation & layout | Retained native editor tabs, split, accordion, breadcrumb and pagination; tab text retained/hidden semantics; ordered pagination requests | Split pointer/keyboard plus carousel/sidebar/navigation-stack integration |
| Commands & feedback | Command button, popup/in-window menus, disabled semantics, OS shortcuts and chooser selection; progress stages; native toast close/expiry and page departure cleanup | Context-menu/nested-menu keyboard and all theme/scale combinations |

The combined native test opens a second independent window, verifies independent
editor values, closes it, cycles editor page unmount/remount three times and
closes the primary window. These tests do not yet prove complete resource/cache
budgets or all component families. They use actual macOS accessibility actions
and OS keyboard delivery to the child process, not bridge-injected click events.

## Commands and results

```sh
./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec dune runtest test/gallery
./scripts/gpuio exec dune build @fmt
python3 scripts/audit_component_catalog.py
python3 scripts/test_gallery.py --section all --images scratch/gallery-images
```

All pass locally for this checkpoint. The expect test verifies latest-state
read-only gating of delayed rating requests, saturated relative bursts and
clear/toggle behavior. A second reducer test verifies current command availability,
bounded notification replacement, stale dismissal rejection and departure cleanup.
The combined eight-section native test reports
`GPUIO_GALLERY_AX_OK: section=all, native actions, state semantics, focus and shutdown`.
Focused picker/overlay/navigation runs also pass. The shell's preview sizing is
logical sizing, not a substitute for native OS display-scale acceptance.

Initial fixture failures were corrected without relaxing behavior: an inactive
background app did not expose its window through AX; native acceptance launches
normally. Multiline editors use AXTextArea, not AXTextField. The combined test
raises the intended first window and waits for editor focus before typing after
a secondary window closes. Every test child is terminated/reaped on failure.
No passing claim is based on the failed attempts or a screenshot alone.
The first feedback fixture incorrectly expected Bonsai sample state to reset on
page departure. The corrected contract preserves that state separately from native
leases, while lifecycle hooks clear transient choosers, modals and notifications.
The corrected focused and combined runs pass, including departure with a live toast.

## Catalog audit boundaries

The immutable source snapshots verify by SHA-256. The structural inventory covers
67 base and 79 component root modules, 73 GPUIX style fields, 22 generic events,
12 intrinsic elements and 13 React export modules. `families.json` assigns every
root module exactly once to 43 owning families and links interfaces/examples/
evidence; `gpuix-styles.json` maps all style names to existing properties/states.
The script rejects missing/duplicate modules, missing API properties and broken
local references. It does not infer behavior from source names.

Detailed family/configuration/event/value review remains explicitly pending.
Known review topics include GPUIX's previously omitted onVisibleRange/onHighlight,
ellipsis-start and extra cursor variants, pointer/wheel defaults, selection scope,
standalone clipboard/automation surfaces and nested editor/document plugin APIs.
Gallery families still missing include managed collections/documents, graphics/
motion/assets, remaining presentation/navigation and desktop
services. OCH-41 remains In Progress; OCH-17's macOS release audit follows it.

Linux build/unit/private-bus/consumer checks remain required. This checkpoint
adds no real Linux GUI qualification; OCH-47 owns that deferred work. Hosted
acceptance for this implementation branch has not yet been run.
