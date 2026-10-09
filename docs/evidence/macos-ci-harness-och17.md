# macOS CI harness corrections — OCH-17 / OCH-41

The hosted [run 37297058441](https://github.com/dakotamurphyucf/gpuio/actions/runs/37297058441)
failed three macOS checks; Linux foundation passed and the dependent fresh-package
receiver was skipped. This remains a failed run. Its tested merge
`7326b23f4f625d2ee1436dfd87c975a42ba4500a` has tree
`b98a97d1b190bb132018b22b848b00984fc5b187`, identical to branch checkpoint
`999e53195418e61d2b0e3e1ac794ecda7e1821eb`. The runner was macOS 15.7.9 arm64
with a 1600×1200, scale-1 display. The original reports, logs, captures and commit
metadata are retained in [the evidence directory](macos-ci-harness-och17/).
The corrections below change test tooling, not production UI behavior.

## Nested document viewport

The hosted test targeted the full layout rectangle of a document whose outer
page clipped part of it. AX can also expose mounted overscan controls before
they are painted inside the document's viewport. The helper now first reveals
the nested viewport by scrolling the outer card padding, then scrolls the actual
document until the requested control's rectangle fits vertically inside it.
It retains the bounded retry count, real wheel delivery, process ownership checks,
paint/focus acknowledgement and native keyboard activation.

A local large-text run with only the outer correction still failed: the AX code
button existed below the document clip, and Return produced no action. That
[failed run](macos-ci-harness-och17/ci-document-profile-large-001/report.json) and
capture are preserved. With both visibility checks, the larger-text walkthrough
[passes all 11 cases](macos-ci-harness-och17/ci-document-profile-large-002/report.json):
code/table actions, inline/block navigation, pointer activation, property update,
remount, independently scrolling plugin controls and source retirement. Exactly
nine expected revision-1 events were observed. The gallery was closed and reaped.
The [focused capture](macos-ci-harness-och17/ci-document-profile-large-002/first-action-focused.png)
shows the code action inside the visible viewport.

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe
python3 scripts/test_macos_document_profile.py --large-density \
  --output scratch/agents/root-20261004-resumed/ci-document-profile-large-002
```

Local machine: macOS 14.5 arm64. Gallery SHA-256:
`21e330ed211f84b7d72c7bf4ebb0dd177991ad1f8ca2b6d38d3620b8bdbef46d`.
Built after `3b5649a` with the concurrently unfinished optional presentation
workload changes present; ordinary gallery does not enable that feature.
[Test source hashes](macos-ci-harness-och17/source-sha256.json) identify this run.
This is a local larger-text reproduction of clipping, not a rerun on the hosted
scale-1 display or proof of automatic offscreen AX focus reveal. VoiceOver was
neither run nor configured and remains explicitly unqualified.

## Palette pixels and display scale

The hosted screenshot correctly painted RGB `(16, 21, 29)` in 2,675 of 14,840
sampled positions (18.03%). The old absolute requirement of more than 3,000
matches assumed the larger Retina image. The checker now requires the expected
palette in at least 10% of sampled positions, independently of backing scale,
while still checking the resolved native label. Wrong painted pixels still fail.

[Offline replay](macos-ci-harness-och17/palette-replay.json) passes the hosted 1×
image and seven previously qualified local 2× images (19.28–19.31%). Each image
hash, dimensions, expected RGB, numerator and denominator is recorded. Two
portable tests cover both palette choices at both scales and rejection of a
matching label with wrong pixels. These tests are now part of both foundation
jobs. This replay does not rerun OS appearance transitions; the original hosted
recovery report records successful restoration to Light.

## Empty IOSurface mappings versus bytes

The original four closed-window samples report **zero dirty, swapped, clean,
reclaimable and wired IOSurface bytes**, with four address-space mappings in every
sample. `vmmap` reports 64 KB virtual size and zero resident IOSurface memory.
The old check treated the nonzero mapping count as allocated bytes.

The corrected gate requires complete recognized nonnegative integer accounting,
zero bytes in every category field, and no increase in empty mappings above the
first closed sample. Any retained byte or growing mapping count still fails.
Six portable memory-report tests pass, including separate byte/mapping failures.
All four original samples pass this narrowly scoped gate on
[replay](macos-ci-harness-och17/memory-replay.json); all 20 recorded raw-file hashes
match the [retained OS samples](macos-ci-harness-och17/original-physical-memory.tar.gz).
The independent actual-renderer Metal counter stays at 5,242,880 bytes after each
close, with zero growth/range, and the original entity-retirement check passed.

These are four smoke cycles, not new full resource qualification. Neither zero
accounted IOSurface bytes nor a stable empty mapping count proves all GPU/native
objects are released. The separate entity, renderer allocation and workload
resource checks remain required. The original failed report is preserved unchanged.

## Validation and remaining scope

```sh
python3 scripts/test_appearance_palette.py   # 2 pass, no desktop access
python3 scripts/test_mac_process_memory.py # 6 pass, no desktop access
python3 -m py_compile scripts/test_gallery.py scripts/test_macos_document_profile.py
```

The actual local document run and offline artifact replays pass. Hosted checks
for these changes, final release packaging and other OCH-17/OCH-41 requirements
remain open; this does not replace the last successful fresh-package checkpoint.
