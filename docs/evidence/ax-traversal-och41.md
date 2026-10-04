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

Commands use `python3 scripts/test_gallery.py --section SECTION --images PATH`.
Each run exits successfully and reaps its application. Per-section logs and PNGs
are retained in the same local session directory. This checks the specified
semantics; it is not a performance benchmark, complete IME/VoiceOver pass or
whole-gallery acceptance.
