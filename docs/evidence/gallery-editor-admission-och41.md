# Gallery editor admission and physical forms — OCH-41

2026-10-04, macOS 14.5 arm64, current local worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. OCH-41 remains open.

## Reproduced failure and repair

The first real-window forms run exited with `Limit_exceeded` immediately after
selecting Text editing. The expanded page instantiated eight primary editors.
Each reserves 8 MiB against the documented 64 MiB/window logical budget, so those
reservations alone exhausted the budget before other retained payload. Opening
the search UI could add two further editors. This was a gallery composition
failure; the native limit remains unchanged.

The page now mounts one selected Bonsai branch:

- Forms & basic editing: form workspace, document title and document body.
- Native input options: password, content hint, formatting and edit filtering.
- Multiline & search: working notes, with query/replacement editors when opened.

All examples remain reachable. Switching groups retires native editor sessions;
the page explains that returning starts fresh sessions. No editor allowance,
resource validation or protocol limit was relaxed.

## Real macOS evidence

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @fmt
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section forms
python3 scripts/test_gallery.py --section editor-groups
```

All four commands pass. The forms run covers **52 geometry cases**, real OS
keyboard editing and three control actions, retained text/undo/redo through
hide/show, and fresh editor state after page retirement. The group regression
visits all groups twice, opens both find and replacement inputs, checks outgoing
editors are absent and returning basic editors start fresh. Both drivers close
their child and observe successful application shutdown. These tests also run
in the macOS CI workflow; no hosted result for this new source is claimed yet.

Testing corrected two fixture assumptions. An AX object can be recreated after
its node leaves the accessibility tree during hiding; retained editing is now
checked through the actual buffer and undo/redo history. Page retirement also
requires waiting for the destination page and checking editor absence before
returning, rather than issuing two asynchronous navigation actions back-to-back.
The new search fixture initially expected single-line roles; the implementation
deliberately uses multiline search/replacement fields (`AXTextArea`).

Logs are retained under `scratch/agents/root-20261004-resumed`: forms-001 records
the admission failure; forms-002/003 expose the fixture assumptions;
`gallery-forms-004.log` and `gallery-editor-groups-002.log` pass. This scoped
foreground evidence does not complete all gallery themes/scales, IME, VoiceOver,
performance, clean-machine distribution or Linux desktop qualification.
