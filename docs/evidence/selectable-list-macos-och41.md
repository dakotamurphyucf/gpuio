# Searchable-list macOS walkthrough — OCH-41

2026-10-08, macOS 14.5 arm64, Apple M1 Max, based on `31c3da40`.
The existing public Searchable list composition now has a focused physical
appearance matrix. No production/library/dependency change was needed. OCH-41
and OCH-17 remain open.

## Qualified interactions

The final repository and installed runs exercise Light/Dark ×
Comfortable/Large/Compact application sizes. Each case performs:

- Down in the query skips the initial section heading. Return delivers Primary
  confirmation and Command-Return delivers Secondary confirmation for entry 0001.
- Space on the focused native list selects the entry. Shift-F10 routes its
  independent Context intent to the Bonsai notice. This fixture does not create
  a context menu or execute the advertised Copy/Pin/Open actions.
- Escape clears the cursor while keeping selection. Real foreground US/ABC-layout
  keys type `remote` in the query editor. Its committed observation drives the
  scoped Eio producer; the fetched heading and record increase loaded membership
  from 1,004 to 1,006 without losing the hidden selection.
- Down explicitly chooses the fetched option, then Return confirms it. A public
  action updates its detail; horizontal/vertical list changes retain the result.
- No matches leaves selection intact. The fixture's deliberate `offline` failure
  appears, explicit Retry succeeds, and New collection replaces source membership,
  clears selection and removes fetched entries. The native query retains `offline`:
  source replacement reruns the current query rather than resetting the editor.
- Leaving Collections removes both the list and query editor from the native
  accessibility tree, checked with bounded waits for asynchronous retirement.

The nonempty query is typed through the guarded foreground OS event route, not
assigned with AXValue. US or ABC input layout is verified without changing it.
Buttons and focus use accessibility operations. Neither driver uses the clipboard
or modifies OS settings, and each owned application closes and is reaped normally.
This establishes the listed paths, not all pointer/disabled-item/range-selection
combinations, cancellation timing, IME composition, VoiceOver, physical frame
timing, search latency or retained-memory limits. The remote/offline producers
are deterministic local Eio fixtures, not network service acceptance.

## Test assumptions and visual review

The initial six-case matrix used the older button-driven query walkthrough and
passed. The next version added context/cancel and actual typing, but incorrectly
expected Return to confirm immediately after Escape had cleared the cursor.
Fetched results were present and selection remained intact; no option was current.
The corrected test explicitly presses Down before Return. This follows the
existing cursor/selection contract; production behavior was not altered to make
the test pass. All attempt logs are retained.

The initial Dark/Large capture was inspected. It showed `offline` still in the
query after New collection. Source review confirms this is intentional: the
search controller preserves the query on independent source replacement, and the
fixture treats a recovered `offline` query as all entries. The final driver checks
this explicitly, and the adjacent walkthrough now explains it. The capture is
not full visual, scrolling or screen-reader acceptance.

## Reproduction and evidence

```sh
python3 scripts/test_gallery.py --section selectable-matrix --images scratch/searchable-root
python3 scripts/test_gallery.py --section selectable-matrix --executable <installed-main.exe> --images scratch/searchable-installed
python3 -m py_compile scripts/gallery_selectable_list.py scripts/gallery_selectable_matrix.py scripts/test_gallery.py
ruff check scripts/gallery_selectable_list.py scripts/gallery_selectable_matrix.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
```

Final native commands use 240-second SIGALRM wrappers that raise through normal
harness cleanup. `searchable-native-003` and `searchable-installed-001` both record
six successful cases and `GPUIO_GALLERY_AX_OK`. The installed executable is the
fresh horizontal-card consumer reused unchanged; no new build was necessary for
this test/documentation-only change. Exact binary SHA256:

- Repository: `d5f449133a950b268e2d9022ad18e3265623598f8cf5dc04a26a467fe0e92bf8`.
- Installed: `f306c78fe75dc623f1ff39df1ce69c0f536a1e300501646df17d950aead1ce5b`.

The [verified archive](selectable-list-macos-och41/reports.tar.gz) contains final
reports/captures/logs and earlier attempt reports/logs. The
[manifest](selectable-list-macos-och41/manifest.json) records file hashes/sizes and
completion summaries. Python syntax, both focused-module Ruff checks, unchanged
legacy-driver lint comparison, actionlint, inventories and whitespace checks pass.
The four-minute focused CI step is new; live Foundation 37789987337 predates it.
Required Linux nongraphical and all remaining catalog/release gates stay intact.
