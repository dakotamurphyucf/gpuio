# List idle redraw investigation — OCH-17

2026-10-08, local macOS 14.5 arm64 / Apple M1 Max, based on `5650ffa5`.
This investigates the third trial's [failed idle check](presentation-list-full-och17.md#current-source-repeat-batch--2026-10-07).
It is diagnostic evidence, not a replacement passing performance repetition.

Temporary instrumentation recorded bounded, timestamped calls to GPUI's
`Window::refresh`, view invalidation and input dispatch. A refinement added the
caller of `bounds_changed`, old/new viewport size and scale, and platform bounds.
The ordinary 96-row smoke traversal used a 60-second idle interval for these
experiments. The native probe retained its existing two-second settling period.
No production budget or rendering behavior was changed. The two source files
were backed up and restored byte-for-byte to Git HEAD after collection; neither
temporary patch is part of the maintained renderer or application API.

| Diagnostic | Test-generated interaction | Idle CPU draws | Metal presented / zero-time | Child exit |
| --- | --- | ---: | --- | ---: |
| Initial control | None | 12 | 7 / 5 | 2 |
| Pointer comparison | 14 moves in seven pairs inside the owned window | 1 | 0 / 1 | 2 |
| Refined control | None | 0 | 0 / 0 | 0 |

The initial control reproduced redraws. Every corresponding refresh call came
from `Window::bounds_changed`; the trace contains no idle input-dispatch entries
that explain them. The current implementation invokes that method from native
movement, resize and activation handling. This identifies the invalidation path
in this diagnostic, **not** which native notification initiated it or whether
the underlying geometry genuinely changed. It also does not retrospectively
identify the cause of the earlier uninstrumented full trial.

In the pointer comparison, the first accessibility query activated AccessKit
and requested one refresh through its activation receiver. The subsequent
14 recorded mouse-move dispatches caused no additional draws. This distinguishes
the test harness's accessibility activation from pointer movement in this
particular fixture; it is not a general guarantee that hovering never redraws.
Pointer locations were checked against the owned application before posting.

The refined control had no idle bounds-change event and no idle draws. Therefore
its additional caller/geometry logging cannot identify the intermittent trigger.
No further repetitions were run to turn that isolated quiet interval into
acceptance. The owner's question about interaction during the original run
remains unanswered at this checkpoint. The control means no **test-generated**
interaction; it is not proof that the wider desktop was untouched.

The first two children failed the unchanged idle assertion before their normal
cleanup/completion records. All children and both batch-owned keep-awake
processes were reaped. Application-level cleanup is qualified only by the
successful refined control's own records. No VoiceOver or clipboard operation,
OS preference change, concurrent compiler or second owned GUI test was used
during these experiments.

The [archive](list-idle-redraw-diagnostic-och17/reports.tar.gz) retains all three
raw logs/reports, both temporary patches, build logs and exact diagnostic drivers.
All 17 files were independently verified against the
[SHA-256 manifest](list-idle-redraw-diagnostic-och17/manifest.json).
The [summary](list-idle-redraw-diagnostic-och17/summary.json) records executable
identities, child outcomes and idle counters without promoting failures to passes.
All four ordinary release presentation executables were rebuilt successfully
after source restoration. Their binaries contain neither temporary idle nor
earlier startup trace markers. The rebuilt list SHA-256 exactly matches the
original `2562490f` executable; the archive includes the rebuild log and hashes.

The remaining question is whether native notifications reflect real desktop
changes or cause unnecessary redraws with unchanged geometry. Preserve the
zero-idle-work requirement while investigating that distinction. Repeated full
list acceptance, collector overhead and the other release gates remain open.
