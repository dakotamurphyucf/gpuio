# VoiceOver attribute-query crash — OCH-17

Local macOS 14.5 arm64, following `ad31f2f5`. This fixes a reproduced crash in
GPUIO's compatibility patch for AccessKit. It does not qualify complete VoiceOver
reading or navigation, and does not complete milestone 07.

## Failure and repair

Starting VoiceOver, accepting its first-run welcome dialog and moving its cursor
to the gallery's rendered document caused the application to terminate. The log
reported an invalid message send to
`-[NSAccessibilityElement accessibilityIsAttributeSettable:]`, followed by a
panic across an Objective-C callback boundary; the child exited with signal 11.
The superclass does not implement that legacy method. Our earlier AXValue
compatibility override had forwarded other attribute queries to it.

A deterministic external macOS regression reproduces the same failure without
VoiceOver: query `AXUIElementIsAttributeSettable` for the rendered document's
`AXRole`. Focus, selected-range and value discovery had already succeeded; the
role query returns `kAXErrorCannotComplete` when the child crashes. Earlier tests
checked AXValue alone and therefore missed the faulty fallback.

`attribute-settable.patch` removes the superclass call. It maps the six supported
writable attributes—value, focus, selected range, selection, expansion and
disclosure—to the adapter's existing modern setter capability rules. Other
attributes are read-only, including application-owned busy state. This preserves
live-node/read-only/disabled policies rather than advertising every setter on
every node. No dependency version or OCaml bridge contract changes.

## Validation and limits

The actual OS selection test now checks mutability alongside its existing actions:

- Rendered Markdown permits focus and selected-range changes; value, role and
  busy state are read-only.
- Read-only code permits focus and selected-range changes, but rejects value
  replacement and role changes.
- Editable text permits focus, selected-range and value changes, but not role
  changes. Unicode selections, backward selections, native Copy, read-only input
  rejection and caret retention still pass.

Command: `python3 scripts/test_macos_text_selection.py --output <fresh-directory>`.
The child exited normally and the clipboard was restored and verified. Gallery
binary SHA-256:
`ff47cb2db0ec1f3a49bb2b3dbeb60330c8db3504d7b7675e67dd30079fe12d29`.

Additional local checks through the isolated toolchain (`GPUIO_JOBS=2`):

- `./scripts/gpuio exec dune build -j2 examples/gallery/main.exe` passed.
- `cargo test --offline --locked -j2 -p gpuio-native --lib --features native-canvas-tests,native-image-tests`:
  **1,175 passed, 2 ignored**.
- Strict workspace/all-targets Clippy with canvas/image/presentation features,
  workspace formatting, and `python3 scripts/test_gallery.py --section collections`
  passed. The latter exercises actual list updates/following, virtual tree/table
  selection, navigation and teardown.

The native tree fixture now checks the legacy mutability route for selection,
expansion and disclosure against its existing enabled/disabled-row expectations,
plus read-only role discovery. Its first two runs passed these semantic/action
checks but failed later history validation. Added diagnostics showed an actual
focused row (`first=12801`, observed/focused row `12983`), contradicting that
retention fixture's no-focus premise. This does not identify the source of the
focus request or establish a production selection-retention defect.

The history phase now clears initial focus and hides the application before its
explicit production layout/paint traversal. It retains every resource assertion;
foreground input/focus/drag suites still run first. One subsequent run stalled in
the earlier focus phase; reactivating only the owned app let the same process
resume. That intervention is recorded in the local notes and retained logs;
this run is not unattended execution or physical-presentation evidence.
It completed with exit 0: **200,000 visits over 100,000 logical rows**, maximum
hierarchy depth 128 and 256 active rows. Evicted selections, list state and
window-owned payloads released; the reported retained cache count after unmount
was zero. The command was
`cargo test --offline --locked -j2 -p gpuio-native --features native-tests --test native_tree`.
The earlier failing logs remain in the bundle. Final checks also passed
`cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-tests -- -D warnings`
and `cargo fmt --all --check` (after formatting the added query expressions).

Two subsequent VoiceOver probes no longer crashed the gallery, but failed to
capture the last spoken phrase through the clipboard. Both test apps exited 0,
and VoiceOver was restored to its initial off state. These are **not passing
reading/navigation tests**. The next probe must inspect VoiceOver's actual
running/caption/shortcut state instead of repeating the same capture attempt.
The startup preference alone is insufficient to prove that speech is running.
No scripted speech output was injected as test evidence.

The probes used Apple's documented [cursor-to-focus command](https://support.apple.com/en-my/guide/voiceover/vo15534/mac)
and [last-phrase copy command](https://support.apple.com/en-gb/guide/voiceover/vo2725/10/mac/26).
The first-run welcome dialog requires explicit handling; the ordinary toggle did
not dismiss it. The test clicked its Use VoiceOver/Turn Off VoiceOver buttons and
did not change the persistent welcome checkbox. A direct preference write failed;
no permissions bypass or authorization-database changes were attempted.

The final patch reconstructs all **13 pinned AccessKit source/manifest files**
exactly from the verified crate archive plus the ordered patches in UPSTREAM.json.
The failing and repaired source, OS reports and check logs are preserved in
[the evidence bundle](voiceover-attribute-query-och17/reports.tar.gz) with its
[SHA-256 manifest](voiceover-attribute-query-och17/manifest.json). Sources are the
dirty tree based on `ad31f2f5`, captured before the delivery commit. No current
Linux GUI, signed distribution, performance or full-release acceptance is implied.
