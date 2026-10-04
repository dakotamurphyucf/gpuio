# Release input and local packaging checkpoint — OCH-17

2026-10-03, macOS arm64, dirty worktree based on `83eb87e`. This is maintenance
and local artifact evidence. It does not close OCH-17, desktop acceptance, license
review, clean-machine distribution, hosted checks or publication.

## Source reconstruction

Fresh isolated outputs were reconstructed using existing cached upstream archives.
The scripts independently verified the recorded archive/patch hashes; no network,
working vendor tree, shared switch or dependency version was changed.

- GPUI core at `a57ba9b17c433ea1ebfdec8f649f4fa5a402d03b`: recursive comparison with
  `vendor/gpui` is exact.
- GPUI Base at `84f57fdfcb4910623fb0bb7f795b077e249f9271`: recursive comparison is
  exact except `vendor/gpui-base/Cargo.lock`. `git ls-files` confirms this is not a
  tracked source input, and `git check-ignore -v` identifies the explicit ignore
  rule. No other source, manifest, patch or license difference was found.
- AccessKit macOS 0.26.3: registry archive hash, **13** upstream files, **9** ordered
  exact patches and **2** license hashes pass `verify_accesskit_macos.py`.

The reconstruction destinations were fresh directories under the implementing
agent's ignored scratch workspace. Exact commands, replacing disposable archive
and output paths as appropriate:

```sh
python3 scripts/vendor_gpui.py --archive ZED_ARCHIVE --output FRESH_GPUI
python3 scripts/vendor_gpui_base.py --archive GPUI_KIT_ARCHIVE --output FRESH_BASE
diff -qr vendor/gpui FRESH_GPUI
diff -qr vendor/gpui-base FRESH_BASE
python3 scripts/verify_accesskit_macos.py --archive ACCESSKIT_MACOS_CRATE
```

A subsequent [Bonsai-only reconstruction](bonsai-reconstruction-och17.md) matches
1,693 entries exactly. The other seven Bonsai-family members, transitive licenses
and remaining release input review are not established by these checks. See
[maintenance instructions](../component-adapters.md).

## Reference bundles

`scripts/package_macos_reference.py` assembles existing Agent Workspace, Component
Studio and Signal Studio executables into fresh `.app`/zip artifacts with checked
metadata, Mach-O dependencies/deployment targets, source/artifact hashes, included
notice hashes and explicit signing/qualification limits. Agent Workspace now
supports the same no-window `--print-info-plist` path as the other two examples,
using `Desktop_package` and Eio output.

All three local development executables:

- Report arm64, macOS deployment minimum **14.4**, and only system framework or
  `/usr/lib` dynamic dependencies, with no runtime search paths.
- Assemble with `--sign ad-hoc`, pass `codesign --verify --strict`, and produce zip
  archives. Extraction into new directories preserves the signature seal.
- Export matching Info.plist metadata through a captured pipe from an outside-
  checkout working directory. This does not run AppKit/GUI or prove clean-machine
  application behavior.

The local notice input included **21 existing repository license/notice files**
plus an explicit review-pending notice. This deliberately remains an internal
qualification artifact: it is not a complete transitive license inventory and
must not be distributed as an accepted release. Full license/NOTICE and embedded
asset review remains required.

Commands used the new packager for each `--app agent_chat`, `gallery` and
`signal_studio`, with fresh scratch output directories and `--sign ad-hoc`.
Then `/usr/bin/ditto -x -k`, `/usr/bin/codesign --verify --strict` and each extracted
executable's `--print-info-plist` verified archive integrity and metadata, without
registering, installing or opening the applications. Per-artifact `package.json`
records SHA-256 values; scratch locations are not build dependencies.

## Observed limitation and checks

One extracted Agent Workspace metadata check redirected stdout to `/dev/null` and
failed with `Assert_failure lib_eio_posix/sched.ml:155:2`. Inspection of the installed
pinned Eio source shows that line asserts the poll result does not contain
`POLLNVAL`; blocking writes await readiness first. The same extracted executable
succeeds through a pipe. An independent no-GUI probe of newly opened descriptors
then found `poll(POLLOUT)` returning `POLLNVAL` (32) for a valid `/dev/null` fd
(`fstat` succeeds), while `select` reports it writable. Fresh regular-file and
pipe fds both return `POLLOUT` (4). This reproduces the platform readiness mismatch
without GPUI or application teardown and explains the assertion's precondition
failure. It is not evidence about the unrelated historical chart latency outlier.
No Eio/switch source was modified. A subsequent narrow public output adapter now
fixes the changed reference exporters; see [output evidence](output-sinks-och17.md).
Stock Eio output remains unchanged. The original archived bundles above predate
that fix, so their hashes must not be presented as repaired artifacts.

- `python3 scripts/test_package_macos_reference.py`: **6 portable tests pass**, for
  metadata identity/path constraints, complete architecture/deployment evidence,
  malformed and external dependencies/search paths, notice preservation/hashes,
  empty notices and symlink rejection.
- The updated Agent Workspace executable builds in the isolated environment.
- Python compilation, repository formatting, CLI help and `git diff --check` pass.
  Portable input checks are wired into both CI
  platforms; no hosted execution of this change is claimed.

See [distribution instructions](../distribution.md) for assembly, limitations and
remaining actual-release acceptance. No Developer ID signature, notarization,
Gatekeeper transfer test, clean-machine execution or physical GUI pass is implied.

## Concurrent-build packaging repair — 2026-10-04

The packager previously inspected the live build output, then copied that path
later. A concurrent rebuild could replace the executable between those steps,
so the packaged bytes could differ from the inspected input. Packaging now copies
the executable into a private temporary directory first and uses that snapshot
for metadata export, Mach-O inspection, the source hash and bundle installation.
The temporary copy is removed on success and failure.

The report and embedded `Build.json` now carry `revision_scope`: the revision and
dirty flag describe the checkout observed at packaging time, not attested build
provenance for an already existing executable. Artifact hashes remain the exact
identity; release evidence must separately associate them with the recorded build.
See [the packaging contract](../distribution.md).

Portable tests now total **eight**, including replacement of the live input during
metadata inspection, consistent inspected/copied/hashed bytes, cleanup and a
failed inspection that creates no output. The existing both-platform CI command
already runs this test file; current hosted execution is not claimed.

```sh
python3 scripts/test_package_macos_reference.py
python3 scripts/package_macos_reference.py --app agent_chat \
  --notices scratch/agents/root-20261003-release-notices/package-snapshot-notices-001 \
  --output scratch/agents/root-20261003-release-notices/package-snapshot-app-002 \
  --sign ad-hoc
```

Both commands pass locally. This uses the existing development Agent Workspace
executable, not a new source build. The deliberately incomplete notices input
contains the project license and an explicit internal-test/review-pending notice.
The resulting artifact must not be distributed as a qualified release.

The arm64 executable declares macOS 14.4 and passes the system-dependency audit.
The bundle assembles, its ad-hoc signature verifies, and extraction into a fresh
directory preserves its executable hash, seal, provenance and notice hashes.
The extracted executable exports matching metadata through a pipe while running
from `/private/tmp`. No GUI, desktop inspection or Launch Services registration
was performed; this is still not clean-machine/Gatekeeper/GUI acceptance.

Final local artifact identities:

| Input/artifact | SHA-256 |
| --- | --- |
| Private input snapshot | `ed37e685312a01007ef2bf5ada61e02526ba1e461001da18904986caef6d314d` |
| Ad-hoc signed packaged executable | `d50ff19eb6da074f310722d3dabe6443c8c7e4aaa196e7e53dbe5f5c0ed38802` |
| Zip archive | `cf44a8329f138d96d822f82ff6d82a6ed13ed8901986e44ca80936929b3a7d44` |

Logs in the same ignored agent directory: `package-snapshot-tests-002.log`,
`package-snapshot-native-002.log`, and `package-snapshot-extracted-002.log`.
Python syntax checks pass. The earlier `*-001` native artifact predates the added
provenance qualifier; use the final `*-002` result above. Neither artifact closes
the remaining licensing, source-build or distribution gates.
