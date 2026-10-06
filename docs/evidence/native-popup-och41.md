# Native OS context popups — OCH-41

Local work based on `8cf5b5f`, macOS 14.5 arm64 (M1 Max), 2026-10-06 UTC.
This is evidence for the opt-in context-popup slice, not completion of the
native-menu catalog or milestone 07.

## Public behavior and ownership

`View.context_menu ~platform:true ~menu child` uses AppKit on macOS and the
existing drawn context menu on Linux. The default remains drawn on both systems.
The wrapper opens on right-click or Shift-F10. The menu shares bounded command
references, checks, disabled state and nested definitions with other command
presentations. AppKit owns its appearance; arbitrary rich View slots are
rejected for this presentation on both platforms.

The [design](../design/native-popup-menu.md) describes retained AppKit ownership,
one active application-wide popup lease, weak retained-view ownership,
main-run-loop scheduling, command-generation validation and native editor
focus/target checks. There are no synchronous OCaml callbacks from tracking.
The paired unpublished protocol appends presentation tag 5; both sides must use
the same source revision.

The standalone [menus walkthrough](../../examples/menus/main.md) explains the
Bonsai, GPUIO and Eio layers and separates normal use from lifecycle probes.
The Feedback gallery wraps its Advance button in the same public platform
context menu. A targeted `test_gallery.py --section native-popup` run passes native keyboard
opening, the disabled section label, Save command delivery and Bonsai toast
dismissal. This does not replace the complete gallery walkthrough.

## Physical macOS checks

`scripts/test_native_popup_macos.py` passes against the repository executable and
an independent consumer linked against staged installed public libraries:

- Real Shift-F10/Enter opens AppKit and increments the OCaml counter.
- Physical right-click is preceded by an ownership hit test. The observed popup
  bounds `(1169, 525, 149, 87)` extend beyond its application's window bounds
  `(534, 299, 660, 549)`, proving the OS surface is not clipped to that window.
- Escape preserves the counter; reopening and keyboard typeahead/Right/Enter
  activate a nested command. The driver waits for AppKit's actual selected rows.
- Copy is disabled for the initially empty editor. After native text selection,
  menu Copy produces the expected text, preserves the draft, and the driver
  restores the previously saved clipboard without logging its contents.
- Updated disabled command state is reflected when the menu is next opened.
- Three separate lifecycle runs observe an open native popup before an Eio timer
  requests window close, removes its owner, or invalidates its command.
  Window close and owner removal dismiss tracking cleanly. An old enabled AppKit
  row cannot invoke the now-disabled registry command. A later checkbox
  roundtrip establishes a subsequent application publication before the test
  asserts the unchanged counter.

All child applications are reaped. These checks do not certify VoiceOver,
physical presentation latency, multiple displays or Linux desktop behavior.

Tested binary SHA-256:

- Repository: `9fca29886ae008f852e0798bbfb749867e34bbe08663b9fa5003ac17154c3251`.
- Installed consumer: `476cc7cf55ee14fa8294d0786899c22e051aaf7c0fdebeca149bdedea7e54437`.

The independent workspace was created using:

```sh
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example menus \
  --workspace scratch/agents/root-20261004-resumed/native-popup-consumer
python3 scripts/test_native_popup_macos.py
python3 scripts/test_native_popup_macos.py --lifecycle close
python3 scripts/test_native_popup_macos.py --lifecycle retire
python3 scripts/test_native_popup_macos.py --lifecycle invalidate
```

The same four driver invocations then passed with `--binary` pointing to that
workspace's `consumer/_build/default/main.exe`. Staging uses an explicit local
installation prefix; it does not change an opam switch or default.

## Failures retained and corrected

The first complete interaction driver used a fixed number of Down keys for the
submenu. After a preceding physical pointer interaction, that did not establish
the expected native highlight and no command ran. The corrected driver uses
native typeahead and observes `AXSelectedChildren` before continuing; assertions
were not removed.

More significantly, the initial implementation called AppKit tracking directly
inside `App::spawn`. Ordinary menu selection passed, but the real close probe
printed `GPUIO_POPUP_CLOSE_REQUESTED` and then timed out waiting for application
exit. The pinned GPUI foreground dispatcher runs on the serial main dispatch
queue. Holding its task inside a nested menu loop prevented queued close work
from progressing.

The correction schedules a one-shot `CFRunLoopPerformBlock` in common modes and
wakes the main run loop, releasing the dispatch queue before tracking. GPUI
borrows occur only before and after AppKit. The unchanged close regression then
passed; the versioned close, owner-removal and stale-command checks also pass on
both binaries. Compilation alone was not used to qualify the repair.

The adapter directly names the already pinned `objc2-core-foundation` 0.3.2 and
enables its block feature. Root and five composed application lockfiles gain only
the two required dependency edges; no package version changed. Full locked,
offline Cargo metadata resolution passes for those five macOS application graphs.
The unrelated standalone resource-audit helper has no native dependency and its
lockfile is unchanged.

## Automated checks and remaining scope

Independent OCaml/Rust fixture bytes, truncation/unknown-tag rejection, opt-in
API/default behavior, rich-slot rejection and context-child admission pass.
The complete protocol suite passes **415 tests**. The final native library passes
**980 tests**, with two existing ignored tests. Native admission passes four
menu-content tests. A production-View snapshot test checks passive labels,
checks, inherited disabling and duplicate command route identities.

Full OCaml `@runtest`, formatting and strict native/protocol all-target Clippy pass
on the corrected code. The full all-example build passed before the scheduling
correction; the current standalone/installed builds and separately recorded
current gallery build qualify their respective binaries. The current gallery build also passes. The final native-library and
gallery-build logs are recorded in the companion archive.

Remaining native-popup work includes programmatic show-at-position, native icon
metadata, overlap, definition replacement and hidden/modal transition cases,
plus consolidated gallery/platform acceptance. AppKit label/check construction
is not a VoiceOver test. Linux uses the drawn route, but this local run is not
Linux execution or desktop qualification. OCH-41 and OCH-17 remain open.

The [archive](native-popup-och41/evidence.tar.gz) and
[manifest](native-popup-och41/manifest.json) preserve implementation sources,
commands, logs, failures and final checks. The initial exploratory dump of the
system menu bar is intentionally not published; it adds no required evidence.
