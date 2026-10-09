# Sidebar desktop walkthrough — OCH-41

2026-10-08, physical macOS 14.5 arm64, source based on `61944bfc`.
The public Journeys sidebar passes its branch-policy, appearance and lifecycle
walkthrough locally and in the previously staged independent gallery consumer.
No production library or application changes were required.

The native checks cover:

- Select-only leaves a closed branch closed; Expand opens it idempotently;
  Toggle alternates via actual foreground Return and Space. The independent
  expansion button does not navigate. Requests update the Bonsai destination.
- Twelve geometry/identity cases per executable: Dark/Light, three scales,
  styling enabled/reset. The fixture reads the actual checkbox state, requires
  nonempty nonoverlapping destination rectangles, and checks the same exposed
  Projects accessibility object across these updates. Representative captures
  were inspected; this is not an exhaustive pixel oracle.
- A guarded OS pointer click selects Observatory. Compact mode hides children
  while retaining the branch's accessible name and stored expansion policy.
  Selecting it in compact mode changes the preference exposed after expansion.
- Offcanvas removes destinations from accessibility, restores selection and
  expansion afterward, and page departure/remount preserves Bonsai state. A
  fresh Archive link accepts real Return after remount. Both apps close normally.

## Corrected fixture assumptions

The first run passed policy and appearance cases, then incorrectly required
`CFEqual` after the entire sidebar became hidden. AccessKit's
`node_updated` in `vendor/accesskit-macos/src/event.rs` removes excluded platform
subtrees and raises `NodeDestroyed`, even for retained GPUIO widgets. Core's
sidebar reconciliation test separately checks that offcanvas Retain causes no
widget removal. The final desktop test records the changed AX object, checks
restored state/actions, and limits identity equality to continuously exposed
destinations. This is a fixture correction, not a widget-retention repair.

Reviewing screenshots from the next local/consumer runs caught inverted styling
labels in the report: the example initializes styling to true. The final fixture
uses the correct reset/enabled order and waits for the actual AX checkbox value.
Earlier logs/reports/captures remain in the archive; only local `003` and consumer
`002` are the final twelve-case appearance evidence. No failure was relabeled.

## Reproduction and provenance

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section sidebar --images scratch/sidebar
python3 scripts/test_gallery.py --section sidebar --executable /path/to/installed/main.exe --images scratch/sidebar-installed
python3 -m py_compile scripts/gallery_sidebar.py scripts/test_gallery.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
git diff --check
```

Local invocations additionally use a 180-second Python SIGALRM raising
`TimeoutError`; the existing gallery runner reaps its child on failure. CI adds
the same standalone section with a three-minute step limit. Syntax, catalog,
example-documentation, actionlint and whitespace checks pass.

Repository executable SHA-256:
`b2453383c1932e4b95831bfc8c4115485e2c68d8e787779dac29f17dcafc3f4e`.
Installed executable:
`a1083299477d1221012a0e7ad1a7a3fca4c7129e269c33f42b3a7c8381539acc`.
The latter reuses the independent consumer built for the
[notification checkpoint](notification-gallery-och41.md); this turn did not
rebuild or claim a new install. Sidebar/application sources have not changed
since that build. Its build log is retained with this evidence.

The [archive](sidebar-macos-och41/reports.tar.gz) preserves logs, reports,
screenshots, final fixture, application source and consumer build provenance.
All members were read back and verified against the
[manifest](sidebar-macos-och41/manifest.json); the
[summary](sidebar-macos-och41/summary.json) identifies the two final runs.
The [code walkthrough](../../examples/gallery/journeys_page.md) traces the
actual Bonsai reducer, decoration and native ownership boundaries.

This closes the recorded desktop gap for these sidebar policies and appearance
updates. It does not qualify VoiceOver, animation frame timing, measured memory,
every nested sidebar option or Linux GUI behavior. Final-source hosted checks
and consolidated OCH-41/OCH-17 release acceptance remain open.
