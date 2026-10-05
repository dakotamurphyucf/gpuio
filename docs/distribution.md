# Reference application packaging

Milestone 07 targets macOS first. Linux source/unit/private-bus/consumer checks
remain required; Linux desktop distribution is deferred to OCH-47. See the
[platform policy](platform-release-policy.md).

The repository can assemble local qualification bundles for Agent Workspace,
Component Studio and Signal Studio. These are inputs to release testing. Assembly
and an ad-hoc signature do not establish clean-machine execution, Gatekeeper
acceptance, notarization, license completeness or a published framework release.

## Build and assemble

Use the isolated repository toolchain, with the intended build profile recorded.
The following produces a development build, not an optimized performance baseline:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/agent_chat/main.exe examples/gallery/main.exe examples/signal_studio/main.exe
python3 scripts/package_macos_reference.py --app agent_chat \
  --notices /absolute/path/to/reviewed-notices \
  --output scratch/package-agent-chat-run-001 --sign ad-hoc
```

Repeat with `--app gallery` and `--app signal_studio` and fresh output directories.
`--executable PATH` supports the corresponding freshly built independent consumer.
Each executable must support the no-window `--print-info-plist` command; rebuild
older Agent Workspace executables before using the packager. Metadata output goes
through a captured pipe. Current reference exporters also use the
[device-safe output adapter](evidence/output-sinks-och17.md) for redirected
character-device output, preserving the pinned Eio runtime. No Launch Services
registration, installation, `open`
command, notification request or GUI walkthrough is performed by the packager.

The script validates the known application identity/executable, versions and
macOS 14.4 metadata. It first copies the input executable into a private temporary
directory, then inspects and packages that same snapshot, so a concurrent rebuild
cannot substitute uninspected bytes between inspection and installation. The
snapshot is removed on success or failure. It inspects every Mach-O architecture/deployment target and
rejects non-system dynamic dependencies, runtime search paths and unsupported
architectures. It does not silently copy Homebrew libraries or raise the advertised
minimum to hide a dependency mismatch. This is a load-command audit, not proof that
all runtime assets and dynamic lookups work on a clean machine.

The output contains the `.app`, a `.zip` and `package.json` with source/packaged
executable hashes, archive hash, architecture/deployment metadata, dependency list,
packaging-checkout revision/dirty state, notice hashes and signing mode. Resources
include `Build.json` provenance. `source_sha256` identifies the private input
snapshot; signing may change the separately hashed packaged executable. The
revision fields observe the checkout at packaging time and do **not** attest which
sources built an existing binary or an independently supplied `--executable`.
`revision_scope` records that limit in both reports. Release evidence must link
the exact executable hash to its recorded build separately.
Output directories must be new; failures after assembly
begins leave `complete=false` evidence rather than overwriting earlier artifacts.
`complete=true` means assembly succeeded, never that release qualification passed.

Signing modes are deliberately limited:

- `unsigned` (default): this tool does not sign the bundle. The input Mach-O may
  already carry the linker’s ad-hoc signature.
- `ad-hoc`: signs the assembled bundle locally and verifies its seal. This is not
  a Developer ID signature and does not submit anything to Apple.

For distribution, the final artifact still needs the project's chosen release
identity/version, reviewed licensing, appropriate signing/notarization workflow
and supported clean-machine acceptance. Those gates remain open; no developer
account or credentials are assumed by this local tool.

## Notices and source maintenance

`--notices` must point to a prepared, nonempty directory of actual license/notice
files. It preserves their relative names and bytes and rejects symlinks. The
provided files are hashed, not declared complete by the tool. Review transitive
OCaml and Rust packages, embedded syntax/theme/font/image assets, copyright and
NOTICE obligations before distributing. `third_party/licenses` alone is not a
complete inventory of linked dependencies. Keep the repository's Apache-2.0
license and applicable upstream notices with the application.

Start a Rust notice audit with the isolated environment and a fresh output:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec python3 scripts/collect_rust_notices.py \
  --target aarch64-apple-darwin --root gpuio-native \
  --supplemental third_party/notice-sources.json \
  --output scratch/notices-native-macos-001
```

This uses offline locked Cargo metadata without compiling or modifying the switch.
`inventory.json` records each package identity, original license expression,
manifest hash, copied text paths/hashes and collection gaps. The metadata and
lockfile are retained alongside it. License alternatives are preserved verbatim;
none is selected automatically. A nested crate never inherits a parent repository's
license by inference. Symlinked and external license files remain explicit review
items, and permission/read errors fail collection. A successful command means
collection finished; `license_review_complete` remains false for every run.

The optional `--supplemental` manifest supplies explicitly attributed workspace
texts. Its paths are repository-relative; text hashes, exact package names,
versions, Cargo sources, license expressions and manifest hashes fence reuse.
Changed applicable manifests or texts fail before output creation. Entries absent
from the selected dependency graph are reported as unused. Each copied text
retains its attribution rationale and source reference, and the exact manifest
is saved in `supplemental-sources.json`. The checked-in mapping covers pinned
workspace and registry package attributions, including Zed Apache symlinks,
nested FFI derives and first-party crates; it does not select alternatives or
approve licenses. Registry source texts use exact revisions recorded in published
crate metadata, with Git blob identities retained and checked alongside SHA-256.

Inventory schema 2 keeps original `review_issues` visible even when supplemental
text is supplied. `packages_without_collected_text` measures text collection gaps;
`packages_with_review_issues` is not a count of unlicensed packages or rejected
dependencies. A schema 1 inventory predates supplemental attributions and must
not be interpreted as schema 2. Neither schema is a release approval record.

The normal/build graph includes build tools and proc macros. It is a conservative
audit input, not an exact list of linked code. Workspace feature unification can
expand it; Cargo can also retain a non-target normal dependency kind on an edge
admitted through a dev dependency. The collector excludes dev-only edges but keeps
such mixed edges rather than silently omitting potentially relevant packages.
Repeat with `--manifest-path`, `--root`, `--target` and any `--features` or
`--no-default-features` appropriate to each independent consumer/backend.

Collect the OCaml/toolchain side from the explicitly named isolated switch:

```sh
python3 scripts/collect_ocaml_notices.py --opam-root .opam-root --switch gpuio \
  --vendor vendor/bonsai --vendor vendor/virtual_dom --vendor vendor/incr_dom \
  --vendor vendor/incremental --vendor vendor/incr_map --vendor vendor/incr_select \
  --vendor vendor/abstract_algebra --vendor vendor/ppx_pattern_bind \
  --output scratch/notices-ocaml-macos-001
```

This includes **all installed packages**, including development/build tools, and
explicitly named vendored trees. It is a conservative input, not a list of objects
linked into a binary. The collector reads installed package metadata, installed
docs and cached source trees; records raw normalized license fields and exact
bytes/hashes; and retains project manifests and source pins. It uses explicit
`--root`/`--switch` arguments for read-only opam queries, changes no switch or
dependency, and rejects an installed-version change during collection before
creating output. Missing/empty/symlink notices remain review issues. Cached trees
are local inputs, not proof of unmodified upstream archives. See the
[OCaml evidence](evidence/ocaml-notices-och17.md) for runtime coverage and open gaps.
Its ten zero-text rows now have an explicit, manifest-hash-bound classification
for compiler markers/selectors, compatibility metadata and the configuration
helper. The [review record](../third_party/ocaml-package-review.json) supplements
the inventory; it does not suppress discovery gaps or approve redistribution.
Repeat for each release environment; do not substitute the macOS switch for the
Linux inventory or infer system/native library coverage from opam metadata.

Review all collected texts, not only flagged packages. Package-local license,
notice, copyright, authors, credits and acknowledgements files/directories are
collected, but this filename-based discovery cannot prove asset or generated-code
coverage. Review and complete OCaml/runtime notices, upstream workspace texts, embedded
syntax/theme/font/image acknowledgements and native/system dependency review
separately. Do not feed an unreviewed collection straight into a release bundle.

The initial [dependency notice evidence](evidence/dependency-notices-och17.md)
records the current macOS collection and its open review items.
The [embedded asset review](evidence/embedded-assets-och17.md) traces syntax/theme,
reference-image, font and shader inputs. The supplemental mapping now includes
readable two-face acknowledgements and [exact Syntect theme-root licenses](evidence/syntect-themes-och17.md). All seven embedded themes were traced, including those not selected at runtime; final derivative attribution/completeness review remains.

See [adapter maintenance](component-adapters.md) for pinned sources, fork patches,
reconstruction and independent lockfiles. Normal packaging uses existing binaries;
it does not mutate opam switches, dependency versions or a vendor tree.

## Acceptance still required

### Local runtime dependency isolation

After assembly, test an extracted archive rather than its build-tree executable:

```sh
python3 scripts/test_macos_package_runtime.py \
  --package scratch/package-agent-chat-run-001 \
  --output scratch/package-agent-chat-runtime-001
```

The command supports each of the three reference packages. It checks the archive
identity/hash and safe entry paths, extracts into a temporary directory outside
the checkout, verifies executable/metadata/notice hashes and the ad-hoc seal when
present, and launches that extracted executable. The app receives a small
environment without development loader overrides. Its `sandbox-exec` profile
denies reads/writes to the checkout, `/opt/homebrew`, `/usr/local` and the user's
`.cargo`, `.rustup` and `.opam` directories. Negative probes verify each existing
denied root before launch. The parent harness retains access to its own code and
reports; no checkout, switch, default, HOME or developer directory is moved.

The real desktop walkthrough checks embedded images and editor selection in the
gallery; chat submission/drafts/tabs/windows/file-picker attachment/theme/close;
and Signal Studio extension/canvas/chart/stream/responsive/remount/close behavior.
It observes the packaged Signal app's existing notification authorization, without
enabling alerts or requesting permission. Runs are bounded, children are reaped
on failure and the original pasteboard representations are restored and verified.
Reports retain exact artifact hashes, deny probes, covered behaviors and errors.

[All three applications have local runtime-isolation evidence](evidence/package-runtime-och17.md).
This is an existing development Mac with specific paths denied. It does not
replace a fresh-machine check, quarantine transfer, Developer ID/notarization,
complete native acceptance or notice review. Test-only bundles with incomplete
notices remain internal qualification inputs.

### Fresh hosted runner

The Foundation workflow stages three ad-hoc archives from its successful macOS
build and transfers them to a separate `macos-15` job. The producer requires the
expected clean checkout; `transfer.json` binds its full Git revision, CI run and
attempt, package metadata hashes, archive hashes and packaged executable hashes.
The build job is responsible for building those inputs at that revision: a hash
alone cannot identify a binary's source. Failed staging or verification leaves
an incomplete manifest.

The receiver installs no project dependencies, restores no dependency cache and
performs no project build. It checks that `_build`, `target` and `.opam-root` are
absent, verifies the transferred identity and hashes, then uses the extracted-app
walkthrough above for each reference application. The hosted image still supplies
macOS, Python and developer/platform tools for the parent harness. Each tested
application runs with the same development-path denials and minimal environment.
The job retains reports and screenshots even when a walkthrough fails.

The transfer scripts can also be checked locally, from a clean built checkout:

```sh
python3 scripts/ci_macos_package_transfer.py stage \
  --directory scratch/runtime-transfer-001 --revision "$(git rev-parse HEAD)"
python3 scripts/ci_macos_package_transfer.py verify \
  --directory scratch/runtime-transfer-001 --revision "$(git rev-parse HEAD)"
```

Local staging does not prove fresh-host execution. Record a completed hosted job
and its exact source/artifact hashes before claiming that coverage. These are
internal qualification artifacts with explicitly incomplete notices and short
artifact retention; this job does not qualify a signed/notarized release or
Gatekeeper behavior after quarantined download. VoiceOver is not exercised.

### Release artifact qualification

Run extracted archives on clean supported macOS environments, with the build tree
and development dependency paths unavailable. Check startup/rendering, assets,
clipboard/focus/IME, accessibility, file/document/URL handling and notification
capability where each example declares it. Exercise ordinary quit, cancellation,
window teardown and repeated use; collect bounded resource/performance evidence.
Test the actual signed distribution artifact after archive transfer/quarantine,
not only the local unarchived app. Record architecture, macOS/display details,
artifact SHA-256 and source revision with each result.

Current local assembly/reconstruction evidence and its limitations are in
[the release-input checkpoint](evidence/release-inputs-och17.md). Required hosted
CI, API/versioning review, source/consumer clean builds and publication remain
separate from these local packaging checks.
