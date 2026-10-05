# Extracted macOS application runtime — OCH-17 / OCH-41

All three reference apps pass real desktop walkthroughs from extracted ad-hoc
signed archives with development-directory access denied. This closes the local
runtime dependency-isolation check; it does not establish fresh-machine,
Gatekeeper, Developer ID, notarization or final release acceptance.

The reusable harness and gallery styling repair are in `3a8a44e`. The gallery's
final archive was packaged on that clean checkout after rebuilding its changed
OCaml example. Chat and Signal binaries were built from the `d380878` application
sources; their package reports correctly record a dirty checkout because the new
test harness was then in progress. Packaging revision fields are observations of
the checkout, not build attestations. Exact input, signed executable and archive
hashes are retained in the package reports below. No vendor or runtime library
source changed in this follow-up.

## What was enforced and tested

Hardware: Apple M1 Max, macOS 14.5 arm64, built-in Retina desktop. The parent test
process verifies archive paths/hashes and extracts into a fresh temporary directory.
The app runs there with the repository, both common Homebrew prefixes, `.cargo`,
`.rustup` and `.opam` denied for reads/writes. All six paths exist on this machine;
negative metadata probes return permission errors. The repository deny rule also
covers its `_build`, `target`, vendored sources and isolated `.opam-root`.
The original HOME is retained, and development loader overrides are absent.

Each run validates extracted executable and notice hashes, Info.plist identity,
the ad-hoc seal, isolated metadata export, ordinary application exit and verified
restoration of the original pasteboard. Only one GUI workload runs at a time.

| Application | Native behavior exercised | Raw evidence |
| --- | --- | --- |
| Component Studio | SVG/raster decoding, fit/theme/scale controls, decode error/recovery, keyboard/button actions, image scope cleanup; source/multiline Unicode selection ranges, copy and read-only protection | [Runtime](package-runtime-och17/gallery-runtime.json), [package](package-runtime-och17/gallery-package.json), [log](package-runtime-och17/gallery.log) |
| Agent Workspace | Search, send and retained draft edits, error/retry, tabs, independent windows, native file picker plus Eio attachment read, theme/command palette, close denial/confirmation and App.run return | [Runtime](package-runtime-och17/chat-runtime.json), [package](package-runtime-och17/chat-package.json), [log](package-runtime-och17/chat.log) |
| Signal Studio | Extension keyboard/actions/disabled/hidden behavior, canvas selection/keyboard/drag/pan/zoom, chart selection, inspector, streaming, compact/wide state preservation, reset/remount and close | [Runtime](package-runtime-och17/signal-runtime.json), [package](package-runtime-och17/signal-package.json), [log](package-runtime-och17/signal.log) |

The first Signal run exposed a harness assumption: its unbundled-binary test
expected notifications to be unavailable, but a real `.app` has a native identity.
The app correctly reported the existing OS authorization. Bundled mode now checks
the UI for the observed authorization state; the original unbundled expectation
remains unchanged. The final run reports `Authorized` and leaves run-completion
alerts disabled. It neither requests permission nor qualifies notification delivery.

## Gallery visual repair

Screenshot inspection found two invisible Copy labels: the example explicitly
set its accent foreground while retaining the ordinary button's accent fill.
The example now supplies its surface background too. Both labels are readable
in the final dark and light screenshots. This is a scoped example styling repair,
not a universal contrast certification.

[Before](package-runtime-och17/gallery-copy-before.png),
[dark after](package-runtime-och17/gallery-copy-dark.png),
[light after](package-runtime-och17/gallery-copy-light.png).
The [Signal Studio wide screenshot](package-runtime-och17/signal-wide.png) was
also inspected and shows its actual canvas, chart and native extension rendering.

## Reproduction and boundaries

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe examples/gallery/main.exe examples/signal_studio/main.exe
python3 scripts/package_macos_reference.py --app gallery --notices REVIEWED_NOTICES --output scratch/gallery-package --sign ad-hoc
python3 scripts/test_macos_package_runtime.py --package scratch/gallery-package --output scratch/gallery-runtime
python3 scripts/test_package_runtime_inputs.py
python3 scripts/test_package_macos_reference.py
```

Repeat assembly/runtime commands for `agent_chat` and `signal_studio` with fresh
directories. Three portable runtime-admission tests pass, including tampered and
incomplete archives, path escape/alias/symlink/device rejection, safe profile
quoting and removal of loader overrides. All eight existing packaging tests,
Python syntax checks, gallery build and formatting pass. The portable checks are
in both CI jobs; the new desktop command has only local evidence at this checkpoint.

The local bundles deliberately contain an incomplete, explicitly test-only notice
set (project license plus a qualification-only marker). They must not be published
as reviewed distribution artifacts. This work does not complete notice review.
A fresh locked native inventory still has 513 packages, 868 copied files with all
hashes independently verified, and 26 missing-text rows. Existing provenance and
classifications remain in [the notice evidence](rust-notice-gaps-och17.md).

Local logs, archives and detailed failed-run evidence are under the implementing
agent's ignored scratch directory using `isolated-{gallery,chat,signal}-*` and
`native-release-notices-current-001`. They are not runtime dependencies. The
denied-directory test still permits ordinary system/user resources elsewhere;
it is not a fresh OS installation or a complete filesystem trace. Full packaged
IME/OS integration, clean-machine/transfer/signing checks, final notice review,
API/release publication and other milestone gates remain open. VoiceOver work
remains on the owner's explicit hold.
