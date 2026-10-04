# Native gallery lookup starvation — OCH-41

2026-10-04, macOS 14.5 arm64. The broader `--section core` run passed status
regions (48 combinations), badges (36 cases and 72 activations) and labels
(48 cases), then failed to find the shimmer source while animation was active.
The subsequent AX dump contained that exact source and role. The overall core
suite did not pass; its already-failed driver's duplicate diagnostic dump was
interrupted, and its child was reaped.

The harness restarted traversal from the root every three seconds inside a
35-second wait. A valid later node could therefore remain unreachable even when
the full wait budget was sufficient. `Mac.wait_find` now passes its original
deadline through one traversal. Direct `find` callers retain their three-second
default. The total wait budget was not increased.

Failure dumps now use batched attributes, at most 200 nodes and a three-second
budget checked between queries, with an explicit truncation marker. Every acquired
child reference is released, including unvisited children. A synchronous macOS AX
call cannot be preempted by this Python deadline; a single blocking call can
overrun it. This is diagnostic hardening, not a production rendering or AX
performance fix.

Validation:

```sh
python3 scripts/test_mac_ax_helpers.py
python3 scripts/test_gallery.py --section shimmer
```

Four portable tests pass: traversal beyond the old per-attempt deadline, bounded
missing-node failure, diagnostic output/reference cleanup, and no further queries
after an OS-call deadline overrun. These tests are included in both CI platforms
without loading macOS frameworks.

The focused real macOS shimmer run also passes: **eight theme/width/direction
cases**, retained identity/geometry, real Unicode clipboard copy and Space
activation, glyph captures, pause/one-shot/source refresh, reduced motion and page
retirement/remount. The application exits successfully. Logs:
`scratch/agents/root-20261004-resumed/{ax-traversal-tests.log,gallery-shimmer-001.log}`.
The wider core run must still be completed; this focused result does not certify
the whole gallery, animation performance budgets, VoiceOver or Linux desktop.

## Broader rerun and tag clipping — 2026-10-04

At source checkpoint `4453cbb`, the full core sequence passed status regions,
badges, labels and shimmer, then markers (18 cases / 21 OS actions) and alerts
(20 cases / 21 OS body actions). It subsequently failed the first tag border
pixel assertion. This remains a failed whole-core run.

A focused run with saved screenshots reproduced the failure: the tag's inner
action was visible, but its bottom border was clipped by the page viewport.
The harness now reveals `Tag preview` (`AXGroup`) rather than its inner button
before sampling. Expected colors and tolerances are unchanged. No production
rendering code changed.

```sh
python3 scripts/test_gallery.py --section tags \
  --images scratch/agents/root-20261004-resumed/tag-images-002
```

The repaired focused run passes 28 theme/variant/outline GPU cases, size groups,
hover/override/unset pixel checks, reordered and absent content, 30 real OS
actions, retained focus/identity, slot/page retirement and state recovery. The
application exits successfully. Evidence: `gallery-core-002.log`, failed
`gallery-tags-001.log` / `tag-images-001`, and passing `gallery-tags-002.log` /
`tag-images-002` in the same local session directory. Remaining core and expanded
gallery families still need acceptance; neither focused pass certifies them.

## Additional physical presentation checks

The same macOS desktop and gallery executable pass these focused checks:

| Section | Observed coverage |
| --- | --- |
| `chat-composition` | 28 theme/variant/alignment cases, 14 GPU surfaces, 28 body and 44 reaction actions, editor retention, Markdown streaming, page cleanup. |
| `chat-list` | 100 logical / at most 12 managed rows, 74 streaming samples, stable draft and history anchor, 16 one-pixel scrolls retaining warm controls, offscreen remount and source cleanup. |
| `descriptions` | 75 packing/axis/width/style cases, 34 GPU cases, native Term/Definition parents, four OS actions, retained rich controls/editors and retirement. |
| `keyboard-labels` | 13 layout/name/identity cases, four GPU cases, registration/disable/platform routing, slot/page retirement. |
| `binding-observations` | 20 live binding/name/identity cases, OS Copy and shortcut invocation, context/config replacement, idle observation silence and retirement. |
| `attachments` | 20 theme/axis/status cases, real keyboard/pointer activation, shielded gaps and disabled actions, image decode failure/recovery, five sizes, identity and page cleanup. |
| `groups` | 64 theme/variant/style/slot cases, 128 pointer/Return actions, real Space input, retained checked state/focus/identity and cleanup. |
| `separators` | 32 theme/axis/pattern/label/width cases and 12 clipping/reset cases, centered geometry, retained checked state/focus/identity and retirement. |

Commands use `python3 scripts/test_gallery.py --section SECTION --images PATH`.
Each run exits successfully and reaps its application. Per-section logs and PNGs
are retained in the same local session directory. This checks the specified
semantics; it is not a performance benchmark, complete IME/VoiceOver pass or
whole-gallery acceptance.

## Standalone links: reveal and cleanup snapshot diagnostics

A standalone links run starts near the top of the large Presentation page. Its
24 fixed 75-pixel reveal steps ended with the target still at screen y=6537,
well below the window. The harness now scales each wheel step toward the observed
target, capped at one viewport, and rereads layout after every event. Ownership
checks and the iteration bound remain unchanged.

The next run reached all 42 expected activations, then failed cleanup against
a static Runtime-page snapshot showing five image registrations. Asset releases
remain in the registry until native acknowledgments arrive; waiting on a captured
label cannot observe later acknowledgments. A focused real-window probe refreshed
the snapshot and observed **10 → 5 → 0 registrations** after switching pages.
The links fixture now explicitly refreshes within its cleanup wait, retaining the
zero-registration requirement. No production resource limit or release behavior
changed. The final whole-links rerun passes all eight theme/content/icon cases,
rich previews, 42 pointer/Return/Space/AX actions, Tab and reverse order, focused
non-stop anchors, disabled recovery and cleanup. The refreshed final snapshots
show six registrations followed by zero; the app exits successfully. This is
`gallery-links-003.log`, with captures in `links-images-003`.

Evidence: `gallery-links-001.log` (reveal failure), `gallery-links-002.log`
(interactions followed by static-snapshot failure), and `link-cleanup-probe-002.log`
(fresh snapshot observations) in `scratch/agents/root-20261004-resumed/`. The first
probe used an incorrect accessible-role assumption for a decorative loading
preview and failed; it is not lifecycle evidence. Brief native/driver samples
during the slow rich-links run are retained there, but do not establish a
performance budget or prove the cause of its duration.

## Cleanup snapshot follow-up

The standalone empty-state run reproduced the same final snapshot race: all
layout/action checks completed, but the Runtime page retained a captured count
of three image registrations. The harness now uses the existing explicit-refresh
wait at all nine remaining image/chart/canvas cleanup sites. Every site still
requires zero registrations and separately checks zero registered source bytes.
No production cleanup behavior or test threshold changed.

The repaired `--section empty` run exits zero on the same macOS 14.5 arm64
desktop: 16 theme/media/alignment/width cases, 10 border cases, 12 typography
cases, intrinsic media and wrapped slots, retained identity/focus, 18 Return/AX
actions, native Space, decoded media, optional retirement and asset teardown.
The final refreshed resource count is zero and the application closes normally.
Evidence: `gallery-empty-001.log` (failure), `gallery-empty-002.log` and
`empty-images-002`, under the same ignored session directory. These runs use
production sources at `984210e`; the passing run includes the cleanup-wait change.

The subsequent shared-reveal regression (`--section tags`, `gallery-tags-003.log`)
passes all 28 GPU cases and 30 OS actions, identity/focus and retirement. A
sequential style/resource queue also exits zero for `styles`, `borders`,
`aspect-ratio`, `assets` and `charts`. Coverage includes all 22 cursor
configurations (not cursor artwork), gradients and bounded accessible text;
border/aspect geometry and retention; SVG/raster readiness and decode recovery;
seven chart families plus mixed layers, keyboard selection, data updates and
bounded source-data pages. Assets and charts both reach zero native registrations
and zero source bytes on page exit. Logs are `gallery-SECTION-001.log`; images
are `SECTION-images-001`. These are functional checks, not timing acceptance.

The next interaction queue stops at `pickers`: opening Dates & colors causes an
`Invalid_tree` rejection and application exit before calendar interaction. This
was a production-path failure: the public calendar constructor encoded a style
declaration on a structural wrapper. The [calendar repair](calendar-content-och41.md)
now passes the physical picker rerun. `gallery-pickers-001.log` retains the original
failure; `gallery-pickers-003.log` records the passing rerun.

## Navigation walkthrough refresh

Overlays pass their focused physical walkthrough (`gallery-overlays-001.log`).
The following navigation run completed the tab checks, then failed because its
accordion assertions still named prose replaced by the retained-editor example.
`gallery_disclosure.py` now exercises that current example: native editing and
Unicode undo/redo survive retained closing/reopening; real Space toggles a
heading; multiple and nonempty-single modes, disabled-item policy, hidden AX
retirement and resetting the buffer after unmount all pass.

An initial fixture asserted `CFEqual` across removal from the accessibility tree.
That was not a valid native-editor identity check: AccessKit removes the platform
AX object on `NodeDestroyed` (`vendor/accesskit-macos/src/event.rs`) even when
the application retains its editor. The corrected test checks the native buffer
and undo/redo history, matching the earlier form retention fixture. It retains
the separate requirement that hidden fields disappear from accessibility.

`python3 scripts/test_gallery.py --section navigation --images
scratch/agents/root-20261004-resumed/navigation-images-003` exits zero, including
the preceding tab retention/menu/reveal checks and following pagination popup,
focus, cancel/submit and boundary-navigation checks. Earlier navigation logs
`001` and `002` retain the stale-text and invalid-identity-check failures. This
refresh changes the fixture, not production disclosure behavior.
