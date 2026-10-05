# Cross-application macOS file drop — OCH-17 / OCH-41

Source checkpoint: `1dfc11796b1c6d1318396964b6b63ba0af90652b`.
Local macOS 14.5 arm64, foreground desktop, public Bonsai gallery with the isolated
stock OCaml 5.3 toolchain. [Report and executable hashes](external-file-drop-och17/report.json),
[gallery trace](external-file-drop-och17/gallery.log),
[independent source trace](external-file-drop-och17/source.log),
[final screenshot](external-file-drop-och17/file-drop.png).

The earlier [drag/drop evidence](drag-drop-och11.md) covers actual AppKit sessions
between windows within a GPUIO process. This new test creates an independent Cocoa
application using `NSDraggingSource` and `NSURL` pasteboard writers. Real system
mouse events start its native drag and release over the public gallery inbox.
There is no GPUI event injection or direct callback invocation. The harness checks
AX ownership before pressing and at the destination; it always releases the mouse
and reaps both owned children on success or failure.

The final local sequence passes:

1. Offer a regular file named `hello λ 👋.txt` and a directory named `nested folder`.
   One receipt reaches Bonsai with Desktop origin, both paths in original order,
   exact UTF-8 bytes and unknown directory metadata (`None`).
2. Disable transfers and repeat the same native drag. The receipt count stays one,
   with no additional dropped payload in the trace.
3. Re-enable transfers and repeat. The receipt count reaches exactly two, with the
   same full path payload. Hover feedback clears after the session ends.
4. Both the file and the nested directory fixture keep their original contents.
   The gallery closes and returns zero; the independent fixture is reaped.

The gallery never opens the offered files. A received path is not an Eio capability
or proof of a filesystem copy/move. AppKit reports copy operation `1` to this source
even for the disabled GPUIO target; that native result is **not** used as application
acceptance. The Bonsai receipt and exact callback trace are the acceptance evidence.
Finder/file promises, cloud-provider materialization, non-UTF-8 OS filenames,
external filesystem transfer and Linux desktop behavior are not covered.

The optional `--trace-input` gallery flag now records offer origin and hex path
bytes alongside the existing phase/gesture trace. Ordinary gallery runs do not log
these paths. No production drag routing or ownership behavior needed a change.

## Reproduction

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe @fmt
xcrun swiftc -module-cache-path scratch/file-drop-modules scripts/macos_file_drag_source.swift -o scratch/file-drag-source
python3 scripts/test_macos_external_file_drop.py --source scratch/file-drag-source --output scratch/external-file-drop-001
```

The new foreground test is part of macOS Foundation CI; its hosted result is still
pending at this checkpoint. Local Python syntax, gallery build/format, Actionlint
1.7.12 and `git diff --check` pass. The first local attempt stopped at fixture window
sizing because its Cocoa window was not resizable; the fixture now enables resizing.
Two subsequent native sequences pass. The final screenshot was explicitly brought
to the receipt and inspected: it shows two received payloads and cleared hover.

No general clipboard writes, Finder interaction or VoiceOver operations occur.
VoiceOver remains on the owner's hold. Broader gallery, accessibility, GPU and
release/distribution requirements remain open.
