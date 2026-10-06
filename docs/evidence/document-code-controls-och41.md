# Independent Code-tab source controls — OCH-41

2026-10-06, macOS 14.5 arm64/M1 Max, code based on `72f4026` with documentation
checkpoint `95b6ba0`. The root gallery and fresh installed consumer both pass
the complete document walkthrough after the fixes below. OCH-41 remains open.

## Application repair

Documentation review found that the Code tab displayed `resources.code` but its
append/reset buttons called the Markdown helpers. Code now has explicit
**Append code** and **Reset code** buttons, a separate six-fragment counter and
helpers targeting that code registration. Initial code status is Streaming,
required by the document append contract. Each append adds an OCaml binding
containing Unicode; counters advance only after successful local source changes.
Reset restores the original greeting fixture and clears that counter. Neither
operation changes Markdown. The native reader remains read-only.

The adjacent [implementation walkthrough](../../examples/gallery/documents_page.md)
now explains the actual helpers, four counters, source statuses, independent
updates and asynchronous publication boundary. The Markdown branch retains its
own append/reset/YAML controls; code exposes its relevant highlight control.

## Native regression and driver repair

The driver verifies exact Code AX text for all six appends, a seventh idempotent
append, reset/reappend from one, and exact original restoration. Native Copy source
checks complete content independently of virtualization. It verifies code append/
reset leave Markdown unchanged, and Markdown reset/append leave code unchanged.
Clipboard text is restored after each source-read helper. The existing native
read-only keyboard checks still verify Backspace and typing cannot replace code.
The remaining document run covers images/links, structure, diff controls,
source reset, three page retirement/reacquisition cycles and clean app shutdown.

The first two runs passed the new code-source regression but failed later in the
existing Markdown reveal helper. Its initial body x-coordinate was captured before
mode-change layout settled: the fixed pointer x was **560**, while the resulting
body x was **593**. That point lay outside the outer preview. Twenty-four wheel
attempts left the body at y=790.5, with its lower edge clipped. Instrumentation
reproduced the same failure; this is not attributed to owner interaction.

The helper now derives its left-gutter point from each iteration's current target
bounds. At x=583, the final run moves y=790.5 → 618.5 → 543.5 and satisfies the same
full-viewport visibility assertion before scrolling the nested reader. No native
scrolling implementation or visibility criterion was changed. Both final root and
installed runs pass, and failed logs/screenshots remain archived.

## Checks and scope

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe @test/gallery/runtest @fmt
./scripts/gpuio exec ocamlformat -i examples/gallery/documents_page.ml
python3 -m py_compile scripts/test_gallery.py
python3 scripts/test_gallery.py --section documents --images scratch/agents/root-20261004-resumed/document-code-controls-images
python3 scripts/test_gallery.py --section documents --images scratch/agents/root-20261004-resumed/document-code-controls-rerun-images
python3 scripts/test_gallery.py --section documents --images scratch/agents/root-20261004-resumed/document-code-controls-final-images
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --run --gallery-section documents --workspace scratch/agents/root-20261004-resumed/document-code-controls-consumer
python3 scripts/audit_example_docs.py
git diff --check
```

Initial Dune invocation failed formatting only; after explicit formatting, the
same build/gallery-tests/fmt command passes. Python compilation, link/inventory
review and whitespace checks pass. The consumer build, catalog preflight and GUI
all pass on the first consumer attempt. No Rust or protocol implementation changed,
so this repair does not rerun or claim a new full native unit suite.

Root executable SHA-256:
`6844e5b8ff2eb0d8d7a315ab84e13c43ff451c9685a5784527eab93fdbb56fdc`.
Installed executable SHA-256:
`401d8d2cf589c8613eb95e74ecbb41d0090880b624a6e700f775374c4e02cddd`.
[Evidence archive](document-code-controls-och41/evidence.tar.gz) and
[per-file hashes](document-code-controls-och41/manifest.json) preserve logs,
source overlay, commands and selected failed/final screenshots.

This establishes the scoped local application behavior and test-driver correction.
It is not whole-gallery, VoiceOver, IME, optimized-performance or release acceptance.
Required hosted checks must cover the final delivered source; the ongoing older
run 37447717604 is not acceptance for this repair. Linux GUI remains deferred OCH-47.
