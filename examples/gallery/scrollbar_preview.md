# Shared scrollbar-description walkthrough

Read [scrollbar_preview.ml](scrollbar_preview.ml) and its [interface](scrollbar_preview.mli).
[collections_page.ml](collections_page.ml) mounts it on **Collections**, rendering
controls/ordinary viewport and sharing `description` with virtual-list previews. `t` contains
controls, viewport and optional `Scrollbar.t`; accessors expose those parts, not a scroll
offset.

`B` is `Bonsai.Cont`, `V` `Gpuio_bonsai.View`, `E` `Bonsai.Effect` and `Scope` Gpuio_eio.Scope.
`graph` hosts state and scope computations. `Expert.Var` provides mutable reactive
mode/busy/status (initially Always/false/instructions); ordinary `B.state` /toggle owns Both
axes, rows 24 and custom/styled/animated true. `let%arr` reads reactive current values to derive
descriptions and views. Constructing effects does not read system preferences or change state.

[Preview_scope](preview_scope.md) creates a visit scope and cancellation callback clearing busy.
`choose` sets mode/status only while scope active and not busy. Native While scrolling
activation executes that effect, updates reactive Vars, derives
[`Scrollbar.create`](../../lib/core/scrollbar.mli), and reconciles every preview using the
shared description without replacing their native scroll owners.
The ordinary viewport uses `V.with_scrollbar`. Managed previews instead pass
`~scrollbar` to their Bonsai component, which reaches the native viewport inside
its layout wrappers; decorating the returned outer layout would not do that.

Use system preference sequences `let%bind`: admits one request, sets busy, awaits
Desktop.scrollbar_preference, then publishes only if scope remains active. Auto_hide maps to
Scrolling, Always_visible to Always; unsupported/error preserves the chosen mode. Controls
disable while busy so another explicit mode cannot race this query. This is a snapshot, not a
subscription; departure prevents late publication.

Cancelling the visit scope does not reset the already published mode or status.
The Bonsai branch retains those values when you leave Collections and return;
`Preview_scope` supplies a fresh request scope on reactivation. A pending reply
from the old scope cannot overwrite that state, while a new explicit read can.

`appearance` builds an Oklab accent→muted gradient thumb, 18-pixel track, base/hover/pressed
geometry and colors. `motion` supplies 0.8s idle hold and 160/240/120ms enter/exit/expand
timings. Turning animated off selects Motion.default (immediate movement, default idle policy),
not a task cancellation. Axis selects displayed bars, not allowed scrolling axes.
`description=None` removes custom bars without replacing viewport ownership.

The ordinary viewport is height 235 with overflow both axes, content width 900 and 24–60 keyed
rows. Add six rows uses a captured setter next value, bounded at 60; it is not a latest-model
reducer. Change bars/appearance and compare retained position, drag/keyboard input and Escape
cancellation. Native GPUIO owns handles/offsets/animation; description owns none. Adapt by
sharing presentation values across independent viewports, validating colors/geometry and binding
asynchronous preference publication to the desired scope. No preference watcher, asset
registration or OCaml animation loop is created.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

These repository-wrapper commands are instructions, not validation performed for this
documentation change. The focused macOS entry point is
`python3 scripts/test_gallery.py --section scrollbars`; it uses the gallery executable.
Compilation does not establish native focus, keyboard, animation or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).

The [native preference follow-up](../../docs/evidence/scrollbar-preference-och41.md#actual-appkit-and-gallery-follow-up--2026-10-08)
checks AppKit's current/legacy/overlay results, applying while scrolled, native
range keys and page reactivation. Process-local test defaults do not modify the
user's System Settings; live global preference changes and animation timing are
separate checks.

The AppKit comparison helper finishes application launch and processes its event
loop for a fixed half-second before taking the expected snapshot. On hosted macOS,
a cold query can return overlay before AppKit resolves the automatic device policy
to legacy. Reports retain both startup and initialized values; the public gallery
result must still match the initialized value exactly. This helper initialization
is test setup, not polling or a delay added to `Desktop.scrollbar_preference`.


The [scrollbar input follow-up](../../docs/evidence/scrollbar-input-macos-och41.md)
records an ordinary-viewport matrix for both themes and three sizes: native keys,
AX range actions, track click/drag, Escape capture cancellation and retained offsets
across description changes. Removing custom bars removes their ranges but does not
reset the underlying viewport. The driver reveals the outer page before querying
clipped bars; those page offsets are separate from the inner viewport offsets.
This is not qualification of every managed owner, VoiceOver or animation timing.
