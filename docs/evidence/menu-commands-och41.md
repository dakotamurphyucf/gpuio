# Positioned menu commands — OCH-41

Local work based on `7092738`, macOS 14.5 arm64, M1 Max, 2026-10-06 UTC.
This qualifies the positioned-controller slice, not completion of OCH-41 or
milestone 07. The [contract](../design/menu-commands.md) defines coordinate,
ownership, admission and cancellation semantics.

## Public interface

`Gpuio_eio.Menu_controller.create window graph` supplies a stable key and an
observer for one `View.context_menu`. Its `command` accepts `Menu.Command.Show`
with a validated `Menu.Position`, or `Close`, returning a typed asynchronous
result. Applications can use the standard controller without Rust knowledge;
`App.Window.Expert.menu_command` is the lower-level adapter operation.

The snapshot captures exact window, node and observer generations. The existing
reconciler rotates menu subscriptions on definition/presentation replacement;
callbacks and styles alone preserve them. Native admission checks this identity
again. The AppKit adapter additionally captures observer identity before deferred
tracking and rechecks it before selection; removing the observer cancels tracking.
The controller's `reset` clears local bookkeeping without being the sole safety
mechanism.

Success acknowledges native admission. Visibility includes a pending OS tracking
lease, and neither the reply nor an observation proves physical display. Menu
selection continues to use the existing asynchronous command registry. The
unpublished paired protocol appends request 23 and result 81; existing tags and
menu visibility events are retained. Both packages must share this revision.

## Actual AppKit qualification

The new [standalone example](../../examples/menu_controller/README.md) separates
its small window launcher from the Bonsai/controller/view component and includes
an adjacent [walkthrough](../../examples/menu_controller/component.md).
The final independently installed public-library consumer passes:

- Show requests at `(100,100)` and `(400,200)` produce popup rectangles
  `[634,332,123,54]` and `[934,432,123,54]`: the requested relative displacement
  is exactly `(300,100)` logical pixels on this display, without clamping.
- A second Show against the current owner returns Busy and leaves the first
  popup in place. A request from a different mounted owner also returns Busy,
  exercising the shared AppKit tracking lease. This is one-window coverage;
  it does not establish every multi-window transition.
- Close completes and dismisses the popup without invoking Run. Reopening and
  selecting Run through native keyboard typeahead/Enter increments the OCaml
  counter once; the editor's draft remains unchanged.
- Detaching the observer during native tracking dismisses the popup. Reattaching
  produces a fresh subscription and permits subsequent opening.
- A snapshot saved before definition replacement returns Stale_menu. The driver
  first observes the replacement subscription and opens the replacement menu,
  so this assertion is not based on a guessed delay or an unaccepted View update.
- Closing an already dismissed current menu succeeds, and the app exits cleanly.

A preceding repository run passes the same sequence before the additional
second-owner probe. The final second-owner case is covered by the installed run.
A separate current-library run of `test_native_popup_macos.py` also passes the
existing right-click/outside-window/submenu/native Copy interactions; that driver
preserves and restores the clipboard. All child processes were reaped.

Final binary SHA-256:

- Repository: `9483d6ae508e88410446d5fb7b06c34079e57d5d979c58f0a8ebe13be96ec5a6`.
- Installed: `d604513b4329614ceea0bb3b5bb46bd3030300849714ece9a8b45b8288ea5df6`.

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example menu_controller \
  --workspace scratch/agents/root-20261004-resumed/popup-controller-consumer
python3 scripts/test_menu_controller_macos.py --binary \
  scratch/agents/root-20261004-resumed/popup-controller-consumer/consumer/_build/default/main.exe
```

Staging uses a local installation prefix, not an opam switch installation.
The required macOS CI popup step now includes this example and driver. Hosted
execution of that addition remains pending.

## Deterministic and build checks

- Full protocol suite: **416 passed**. Independent literal OCaml/Rust bytes cover
  request/result tags, float layout, request truncation, trailing bytes, invalid
  tags/correlation, nonfinite coordinates, bounds and boundary values.
- Full native library: **981 passed, two existing ignored**. The new TestPlatform
  test uses the production drawn-menu View to check captured coordinates,
  Busy, wrong observer, invalid position, inactive rejection, idempotent close,
  observer replacement and initial/open/closed observations.
- OCaml expect tests check public coordinates/bytes, exact snapshot identity,
  definition retirement, correlation, the 64-request bound and once-only
  completion despite duplicate or post-close replies.
- Full `dune build @all @fmt @runtest` passes with the new library/controller and
  updated low-level examples. The subsequent example-only second-owner addition
  passes its own build/format check and fresh installed native run.
- Strict native/protocol all-target Clippy, Rust formatting, Python compilation
  and workflow actionlint pass. No dependency version changed.

The [source/log archive](menu-commands-och41/evidence.tar.gz) and
[manifest](menu-commands-och41/manifest.json) preserve exact commands, final logs,
source files and corrected build failures. An early diagnostic GUI run is marked
as using a pre-final-follow-up binary and is not substituted for the installed
qualification above.

## Limits and remaining work

Native icon metadata, multi-window overlap/focus transfer, native editor target
changes and consolidated catalog/release acceptance remain open. The drawn
position path has production TestPlatform evidence; this local run is not Linux
execution or graphical qualification. VoiceOver, frame timing and performance
acceptance are separate. The older live CI run does not qualify these new changes.
