# Disclosure desktop walkthrough — OCH-41

On 2026-10-08, the public gallery's disclosure example passes six combinations
of Light/Dark appearance and Comfortable/Large/Compact application sizing on
macOS 14.5 arm64. The same checks pass in a fresh consumer built against staged
installed public libraries. This is component interaction and lifetime evidence,
not complete OCH-41 or release acceptance.

## Behavior exercised

Each case remounts the Navigation page to obtain a fresh Bonsai model and then:

- Seeds a Unicode draft through AX, replaces it using actual keyboard input,
  closes/reopens Identity and verifies retained native undo and redo.
- Checks expanded state and absence of the hidden text area from the AX tree.
  Native buffer retention does not require the same AX object after hiding.
- Uses actual Space on a focused heading to close it; exercises optional single,
  multiple and required-single expansion through application controls.
- Checks that disabled Behavior is unavailable, then turns off Keep drafts while
  Identity is closed and verifies the initial seed in its new native buffer.
- Uses actual Home, End and Down to navigate headings without changing expansion;
  Down skips disabled Behavior. Whole-group disabling makes all three headings
  unavailable while preserving the expanded Identity panel, and recovery retains
  its content.

The driver checks the US/ABC input source without changing it and sends foreground
keyboard events. Model controls use macOS AX actions; this is not an all-pointer
walkthrough or an IME test. Comfortable-size Light/Dark captures were inspected
for the native draft and separated headings. Application sizing is not a claim
about switching physical monitor density.

## Reproduction and evidence

Based on `c1d2a477`, with test, CI and documentation changes only:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
python3 scripts/test_gallery.py --section disclosure --images <fresh-output>
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace <fresh-workspace>
python3 scripts/test_gallery.py --section disclosure \
  --executable <fresh-workspace>/consumer/_build/default/main.exe --images <fresh-output>
python3 -m py_compile scripts/gallery_disclosure.py scripts/test_gallery.py
ruff check scripts/gallery_disclosure.py
python3 scripts/audit_example_docs.py
python3 scripts/audit_component_catalog.py
```

The local desktop invocations use a 180-second alarm that raises through the
driver's cleanup path. Both final runs close and reap their owned application
normally. The independent build also passes both native catalog-schema checks.
The initial matrix attempt used the wrong sidebar label, `Navigation`, rather
than `Navigation & layout`; it failed before exercising disclosures. Correcting
the driver produced six passing baseline cases. The final runs add heading and
group-disabled assertions and reveal the editor before captures. No production
code or expectation was changed to make a behavior failure pass.

[Reports, captures and build logs](disclosure-macos-och41/reports.tar.gz) and the
[manifest](disclosure-macos-och41/manifest.json) preserve that failed attempt,
the intermediate pass, both final passes, source hashes and exact binaries.
The example walkthrough and family ledger link these scoped results, and
Foundation now runs `--section disclosure` separately from the baseline full
Navigation check.

The native reveal-motion tests retain their own [evidence](disclosure-presentation-och41.md).
This walkthrough does not establish intermediate animation frames, reduced-motion
timing, full rich-header geometry, IME, VoiceOver speech, resource budgets or
clean-machine distribution. Required final-source hosted/Linux checks and the
consolidated release gates remain open. No Linux GUI acceptance is claimed.
