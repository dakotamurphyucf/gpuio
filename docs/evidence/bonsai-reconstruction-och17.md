# Bonsai source reconstruction — OCH-17

Checkpoint 2026-10-04, macOS arm64. The initial check below establishes Bonsai
itself from a dirty worktree based on `83eb87e865c86717a8bc51b9db6fe1f379d909a9`.
The later [complete-family check](#complete-family-reconstruction-after-access-was-restored)
covers all eight roots. Neither establishes release readiness; OCH-17 remains open.

## Verified source

The existing cached upstream archive matches the recorded SHA-256:
`11de33880bb22cff4c95966b21c108773255ba8ccb1c46fb1f071eb337a59ab4`.
The pinned commit is `e929674585a67818734b06e12ea328872d185970` and the maintained
patch SHA-256 is `4121a7e06578b9747e42eefb37cb3a363b69d1c34b98080dd90ef0da9b76e98b`.
No pin, patch, manifest hash, vendored source or switch changed in this checkpoint.

```sh
python3 scripts/vendor_bonsai.py \
  --archive-dir scratch/agents/root-20261003-release-notices/bonsai-archives-001 \
  --package bonsai \
  --output scratch/agents/root-20261003-release-notices/bonsai-reconstructed-001
```

The output matches all **1,693** live file/symlink entries in `vendor/bonsai`,
including content hashes, symlink targets and executable bits. There are **no
exclusions**. Logs: `bonsai-reconstruct-001.log` and
`bonsai-reconstruct-compare-001.log` under the same ignored agent workspace.
The local archive path is evidence, not a normal build dependency.

## Reconstruction tooling

The script now accepts a fresh `--output` and a hash-verified `--archive-dir`,
so maintainers do not need to move the live vendor tree or duplicate scripts and
manifests merely to run maintenance checks. Explicit repeatable `--package`
selection supports a scoped check; the default still reconstructs all family
members. Offline mode never downloads a missing archive. Ordinary runs preserve
the source manifest; `--record-archives` remains a deliberate source-update mode.
Existing output paths, including dangling symlinks, are rejected.

`python3 scripts/test_vendor_bonsai.py`: **five portable tests pass**, covering
full and selected offline fixture reconstruction, preservation of the live tree
and manifest, missing archives without partial publication, archive/patch hash
corruption, and existing/dangling output refusal. The fixture child's PATH has a
patch program but no curl. These tests validate admission behavior; their small
synthetic sources do not substitute for reconstructing the real upstream family.
Log: `bonsai-vendor-tests-001.log`. The test is wired into both CI platforms; no
new hosted execution is claimed. Python syntax, all eight recorded patch hashes,
five edited-document relative links, workflow YAML/required-step placement and
`git diff --check` also pass. Runtime sources are unchanged, so runtime tests were
not repeated for this tooling-only checkpoint.

## Complete family reconstruction after access was restored

On 2026-10-04 the session received network access. All seven previously missing
archives downloaded from their pinned GitHub commits and matched the recorded
SHA-256 values. The cached Bonsai archive was verified again. Reconstruction with
the maintained script and patches then matched every live file/symlink entry and
executable bit, with no exclusions:

| Package | Matching entries |
| --- | ---: |
| bonsai | 1,693 |
| virtual_dom | 176 |
| incr_dom | 193 |
| incremental | 129 |
| incr_map | 113 |
| incr_select | 17 |
| ppx_pattern_bind | 17 |
| abstract_algebra | 10 |
| Total | 2,348 |

```sh
python3 scripts/vendor_bonsai.py \
  --archive-dir scratch/agents/root-20261004-resumed/bonsai-archives \
  --output scratch/agents/root-20261004-resumed/bonsai-reconstructed
```

The command exited successfully. `archives.json`, `bonsai-reconstruction.log`
and `bonsai-comparison.json` in that agent workspace retain the inputs, patch
output and zero-difference comparison. Pins, patch files, the source manifest,
live vendor files and the switch were unchanged. This closes the eight-package
source-reconstruction gap; it does not certify licenses or clean-machine builds.

## Earlier access limitation and remaining release work

Only the Bonsai archive was available in the inspected local caches. Fetching
`virtual_dom` failed with `curl: (6) Could not resolve host: codeload.github.com`.
At that earlier checkpoint, exact-hash opam cache lookups did not locate
the other seven archives. Reconstruction was then unverified for:

- `virtual_dom`, `incr_dom`, `incremental`, `incr_map`, `incr_select`,
  `ppx_pattern_bind`, and `abstract_algebra`.

The later complete-family check above supersedes that availability limitation.
Do not substitute current vendor files or re-created archives for the original
hash-verified inputs in future checks.
Current builds and the installed consumers have separate evidence; they do not
prove source reconstruction, clean-machine GUI execution, licenses or release
publication. Other milestone gates remain in [status](../status.md).
