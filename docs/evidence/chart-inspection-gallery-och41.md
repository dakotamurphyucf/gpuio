# Public rich inspection gallery

OCH-41, 2026-10-06. Local macOS 14.5 arm64 (Apple M1 Max), after `a3c5f1a`.
The public gallery now composes structured rows, a Bonsai action and a native
editor through ordinary OCaml APIs. This is scoped feature qualification;
OCH-41 and OCH-17 remain open for the rest of the catalog and release gates.

## Implementation and ownership

[`Chart_content`](../../examples/gallery/chart_content.md) separates the reactive
controller from pure controls and content derivation. It builds Bonsai mode/count
state and one `Gpuio_eio.Text_input` controller. Pie entries use stable typed
slice IDs and `Presentation.Chart_inspection` title/Weight/Share rows. Only ID 1
receives the editor and action; the controller is never placed twice. Card and
Overlay use the same child keys. Returning to Rows/Native or leaving the page
destroys the native draft while the page-owned Bonsai activation count persists.

The chart page still owns its single scoped data registration. Other family
demonstrations retain their native summary. This adds no native widget, schema,
dependency or synchronous callback. The guide explains `let%arr`, setter effects,
typed targets, keys, desired-versus-accepted publication timing and native state.
The pie fixture's Update chart samples action deliberately keeps the same data;
its invocation here is not evidence of a changed native publication. Separate
[adapter checks](chart-inspection-editors-och41.md) exercise actual source changes.

## Local evidence

- Full `dune build @all @runtest @fmt -j 2` passes in the isolated environment.
- The root gallery walkthrough passes formatted rows, native Home/Tab entry to
  an uncommitted preview, Space/button effects, typing and Backspace, light/dark
  focus and draft retention, Comfortable/Large/Compact draft retention, and
  Card/Overlay focus and draft retention.
- Disabling the chart hides inspection children from accessibility and keyboard
  input; re-enabling and previewing recovers the retained draft. Original-data
  browsing hides content and preserves it on return. Removing interactive entries
  and leaving/re-entering the page destroys native drafts but retains the Bonsai
  count. Final registration counts and source bytes return to zero; normal close
  exits successfully.
- Captured light/dark rows, Card and Overlay images were inspected for visible
  content. Screenshots supplement actual input assertions; this does not certify
  physical IME candidate panels, VoiceOver, every arbitrary child widget, Linux
  desktop behavior or physical presentation latency.
- A fresh installed consumer passes the same complete native walkthrough and
  zero-resource cleanup. Its independent build also passes the counter/document
  profile catalog handshake. Root and consumer executable hashes are in the logs.

The first walkthrough expected a disabled editor to remain accessible. The chart
instead retires inspection itself, correctly hiding its children. The test now
checks absence, rejected hidden typing and draft recovery. A later scale check
attempted to focus an editor below the scroll viewport. Its failure image showed
the chart clipped at the bottom; the corrected driver explicitly reveals the
chart, creates a fresh preview and then checks the retained draft. Scale checks
therefore do **not** claim uninterrupted keyboard focus while clipped.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j 2
python3 scripts/test_gallery.py --section chart-content --images scratch/inspection-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace scratch/inspection-consumer
python3 scripts/test_gallery.py --section chart-content --executable scratch/inspection-consumer/consumer/_build/default/main.exe
```

The consumer command stages public OCaml packages into an isolated prefix and
builds a fresh example with the pinned composed Rust backend. It does not install
into an opam switch or establish clean-machine/signing acceptance. The dedicated
`chart-content` section is also invoked by the full Charts walkthrough.

The [raw logs and screenshots](chart-inspection-gallery-och41-logs.tar.gz) and
[SHA-256 manifest](chart-inspection-gallery-och41-manifest.json) contain 27 members
(5,097,816 uncompressed bytes). They include both failed preconditions, corrected
root/consumer runs, exact commands/exits, executable hashes and tested source
hashes. The three copied consumer implementation/interface files match the root
sources byte for byte. Hosted run 37549499328 targets earlier `54173d2`; it cannot
qualify this addition. No platform or release gate is waived.
