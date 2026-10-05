# Closed-window macOS physical memory — OCH-17

Status: a real native graphics retention problem is reproduced; candidate repair
passes a four-cycle physical smoke and native checks. Full repaired repetitions
are pending. Earlier RSS-only passes do not certify physical-memory acceptance.

## Before repair

The optimized lifecycle executable at the existing source before the platform
repair has SHA-256 `8c498d6c31b1fdfa76a7a94c97a6d2a81f74c2a2ace77ecc1b98e0da3f8ac623`.
The collector is `115cf91930262595cc183f3a8148aa208f9e244b`, clean at launch.
The same M1 Max / 32 GiB / macOS 14.5 reference desktop ran one visible window
at a time, with no other owned GUI or compiler. Source review and later edits
continued while the preserved executable ran. No changes to system settings,
GC forcing or cache purging were used.

```sh
python3 scripts/measure_resource_lifecycle.py --build-profile release --physical-memory --check-budgets --timeout 600 --output scratch/agents/root-20261004-resumed/physical-lifecycle-full-001
```

All 33 warm-up/measured checkpoints retired application registrations and queues.
Peak RSS was 135,053,312 bytes; its final-ten growth was 7,585,792 bytes, below
the predeclared RSS target. In contrast, settled OS footprint grew from
132,041,344 to 2,524,518,528 bytes. Its final-ten growth was 660,522,112 bytes.
IOSurface regions increased by exactly three per cycle, ending at 99, with
1,562,001,408 dirty category bytes. This observation is a retention failure,
not a physical-memory pass despite the collector's `complete=true` and RSS pass.
That flag describes completed collection; the original invocation had no
closed-surface regression gate.

The [complete report](physical-memory-before-och17.json) preserves all values.
The [raw artifact archive](physical-memory-before-artifacts-och17.tar.gz)
contains application output and each cycle's unchanged footprint JSON,
footprint/vmmap stdout/stderr and tool metadata. Report artifact hashes permit
checking the raw files. The OS tools inspect only the exact waiting child PID,
within six seconds per tool, before the next cycle is acknowledged. Tool
warnings/errors or identity/unit/category failures reject collection.

## Repair and preflight

The [GPUI macOS ownership adaptation](../design/gpui-macos-adaptation.md)
breaks a strong native-view/accessibility-adapter/window-state cycle before
native-window destruction, releasing the adapter outside the state mutex.
All 17 files of the new platform vendor reconstruct exactly from the same pinned
upstream revision and separate recorded patch. No dependency versions changed.

The first repaired optimized four-cycle smoke has physical footprints
55,429,696 / 73,304,832 / 60,328,704 / 77,368,064 bytes and no IOSurface category
in any closed checkpoint. Graphics accounting is about 2.4 MB after the first
cycle. The executable SHA-256 is
`6b4255c874450d7e60ed5b4865f4489557dca37303dd8db3d4149d48c782a928`.
This short smoke supports the candidate repair but does not replace the full
repeated lifecycle audit. Native 928-test suite passes (two existing skips);
portable composition, standalone-manifest and physical collector checks pass.

Before full repaired runs, `--check-closed-surfaces` now explicitly rejects any
nonzero IOSurface category accounting/regions after a closed checkpoint. This
regression targets the observed native-window retention, without inventing a
physical-footprint growth threshold after the failure. Physical footprint,
RSS, native entities and all GPU resources remain distinct. OS category absence
alone is not a complete native-entity/Metal audit or a physical-presentation test.
