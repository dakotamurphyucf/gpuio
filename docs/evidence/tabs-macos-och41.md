# Native tab gallery walkthrough — OCH-41

2026-10-08, physical macOS 14.5 arm64, base `fefa6518`. This adds scoped desktop
evidence without a production change. It does not qualify full workspace,
VoiceOver, motion timing or release acceptance.

The existing `--section navigation` walkthrough first passed unchanged, including
tabs, disclosure and pagination. Its tab portion is now extracted byte-for-byte
into `gallery_tabs.exercise_basic`; Navigation still invokes that helper before
the existing disclosure/pagination tail. The separate `--section tabs` extends
it with actual foreground keyboard and geometry checks.

Both the repository executable and the installed consumer pass:

- Editing Notes, switching between rich/plain labels, all five variants and
  custom target styles while retaining the edited native value.
- Switching Notes/Draft with the inactive panel absent from accessibility and
  restoring the retained Notes draft on return.
- Independent close controls, reverse/restore and toggled width limits.
- Selecting Archive without moving the viewport, explicitly revealing it,
  keeping the fixed restore suffix stationary, and selecting Plan through the
  all-tabs menu without an implicit reveal. Closing/restoring Archive preserves
  the separate Notes editor.
- Thirty cases combining five variants, Dark/Light and Comfortable/Large/Compact.
  The configured Notes/Draft targets retain widths 160/140 and height 40, align
  without overlap, and actual Right/Left input switches the active panel while
  preserving the edited Notes draft. US/ABC input layout is checked first.
- Page departure removes the Notes editor; returning creates its initial text.
  This is distinct from the retained inactive-panel lifetime. Both apps close
  normally.

The first focused local run passed. Final local `002` adds an explicit current
variant-label assertion and page-remount check; it and installed `001` pass.
No failed desktop attempt occurred in this checkpoint. The consumer was built
for the [scroll adapter checkpoint](macos-scroll-phases-och41.md) and reused here
because application/native code is unchanged; this is not another install claim.
Binary hashes in each log match that checkpoint.

## Reproduction and checks

```sh
python3 scripts/test_gallery.py --section navigation --images scratch/navigation
python3 scripts/test_gallery.py --section tabs --images scratch/tabs
python3 scripts/test_gallery.py --section tabs --executable /path/to/installed/main.exe --images scratch/tabs-installed
python3 -m py_compile scripts/gallery_tabs.py scripts/test_gallery.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
git diff --check
```

Local runs use a 180-second exception/cleanup wrapper. The dedicated Foundation
step has a three-minute limit. Python syntax, both audits, actionlint and
whitespace checks pass. The public
[page walkthrough](../../examples/gallery/navigation_page.md) explains the
actual state/controller lifetimes; the
[tab content walkthrough](../../examples/gallery/tab_content_preview.md) traces
independent selection, reveal and close actions.

The [archive](tabs-macos-och41/reports.tar.gz) retains the full navigation log,
both focused local logs/reports, installed log/report and exact final fixture and
application sources. All members match its [manifest](tabs-macos-och41/manifest.json).

The forty-pixel targets are explicitly configured logical sizes, not a claim
that all tab metrics ignore application scale. These AX/key checks do not
establish every painted color, native icon pixels, continuous indicator motion,
screen-reader navigation, capture/scroll ownership or resource budgets. Full
split-group desktop coverage and other workspace/release requirements remain
open. Required hosted/Linux checks are separate; Linux GUI stays deferred.
