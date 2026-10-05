# Public document profile on macOS — OCH-41

Source `445af08`, macOS 14.5 arm64, real foreground gallery using the independently
packaged `gpuio-example-document` Rust profile and public OCaml bindings. The profile
package and native host implementations are unchanged by this checkpoint. The
only gallery addition is opt-in typed event tracing for this walkthrough.

[Report and binary SHA-256](document-profile-macos-och41/report.json),
[exact event trace](document-profile-macos-och41/application.log),
[walkthrough](document-profile-macos-och41/walkthrough.log),
[inspected native controls](document-profile-macos-och41/native-controls.png),
[visible focused code action](document-profile-macos-och41/first-action-focused.png).

## Observed behavior

- Enable the registered native document profile through the public gallery control.
  After wheel-scrolling the mounted controls into view, native Enter activates the
  code and table action renderers, delivering `Inspect_code` and `Summarize_table`.
- Native Enter activates the inline review plugin. Tab moves from that plugin to
  the block review card without activation; Enter activates the card.
- Change the profile properties through Amber code highlights, waiting for the
  native checkbox state to acknowledge the update. A system mouse click on the
  visible card delivers one more `Open_card`. The inspected capture also shows
  amber code text together with all four independent native controls.
- Remove the profile, observe both review controls disappear, then remount it.
  The inline native button becomes usable again and emits one `Open_badge`.
- Page departure eventually reports zero documents, zero registered source bytes
  and zero image/chart/canvas registrations. The gallery closes and exits zero.

The complete application trace is exactly six Data events in the expected order:
`Inspect_code`, `Summarize_table`, `Open_badge`, `Open_card`, `Open_card`, `Open_badge`.
All report installed source revision 1. Focus, Tab, property updates, profile
removal and navigation add no activation events. The test owns and reaps only its
spawned gallery process. It uses the foreground OS keyboard route and checks the
pointer target's process before clicking.

## Test corrections and scope

Early attempts stopped at the first action: it was present in the AX tree but
still clipped below the viewport. Changing the key delivery route and waiting
for another paint alone did not fix that setup. The test now reveals the actual
control in both the document and containing gallery viewport before focusing and
activating it. It also waits for GPUI's focused-element paint before keyboard
activation. No production keyboard/drag/runtime patch was made or claimed.

This demonstrates the shipped package's code/table action slots and inline/block
NonText plugins in a real macOS application. It does not establish arbitrary
plugin composites, a plugin-owned independent scroll area's keyboard reveal,
physical trackpad momentum, automatic reveal from an offscreen AX-only focus
request, rich Markdown text selection, or VoiceOver coverage. The [independent
scroll ownership test](document-profile-scroll-och41.md) retains its TestPlatform
scope. No VoiceOver operation, OS setting or general clipboard write occurs here.
Native resource counts are not a GPU/process memory census.

## Reproduction

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt
python3 -m py_compile scripts/test_macos_document_profile.py
python3 scripts/test_macos_document_profile.py --output scratch/document-profile-native-001
```

Build/format, Python syntax, Actionlint 1.7.12 and `git diff --check` pass. The final
walkthrough passes at the clean source checkpoint above. The added macOS CI step
still needs its hosted result. Local prior failed-run diagnostics are retained in
ignored `scratch/agents/root-20261004-resumed/document-profile-runtime-*`; they are
not build dependencies. OCH-41 and OCH-17 remain open.
