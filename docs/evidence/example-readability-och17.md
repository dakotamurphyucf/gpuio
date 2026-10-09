# Example readability — OCH-17 / OCH-41

Local macOS 14.5 arm64 validation, 2026-10-05. Source base `bdab7f1` plus
the archived source changes. This addresses the owner's request to make the
examples easier to navigate and distinguish Bonsai from GPUIO.

The counter keeps its three small parts in one file: stateless `counter_view`,
reactive `component`, and application startup. Component Studio separates
`Application` (runtime/services), `Component` (Bonsai observations/effects),
and `Shell` (GPUIO layout with explicit snapshots/actions). Agent Workspace
separates startup from its self-test and metrics runners. Signal Studio separates
application ownership, the Bonsai component, stateless UI, integration checks and
the task-to-UI adapter. Each example has a README reading path and `.mli` contracts
for extracted modules. Descriptive aliases identify the relevant libraries.

This is an application organization change, not a new public framework API.
The chat workspace still contains substantial reactive composition and Signal
Studio's application module still assembles several services; the small counter
is the recommended first example. CLI modes and native resource ownership remain
part of the larger examples' existing contracts.

## Validation

The following command passes using the isolated repository environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j 2 \
  examples/getting_started/main.exe examples/gallery/main.exe \
  examples/agent_chat/main.exe examples/signal_studio/main.exe @runtest @fmt
```

Independent public-library builds pass with `scripts/test_extension_consumer.py`
for `getting_started`, `gallery` and `signal_studio`, each in a new scratch
workspace. This also verifies that the extracted modules are copied into consumer
builds. All three larger apps' `--print-info-plist` modes produce parseable package
metadata with their expected identities.

Native checks pass:

- Installed starter Increment (three successive updates), Reset and normal Close.
- Agent Workspace and Signal Studio `--self-test` modes, including the installed
  Signal Studio document-path branch with a temporary file.
- Installed gallery `--section shell`: navigation, slider increment, real
  keyboard editing/submission, retained drafts through theme/scale changes,
  independent second-window state and repeated page teardown/remount.
- Installed gallery `--section feedback`: the existing complete command/menu,
  rich palette, keyboard/focus and notification walkthrough.
- Installed gallery custom-chrome window-lifecycle walkthrough: native window
  commands and retained editing.
- Installed Signal Studio walkthrough: extension interaction, canvas selection,
  keyboard/drag/pan/zoom, chart selection, streaming, responsive layout, remount
  and OS close.

The targeted batch closes/reaps its children and restores the clipboard
representations saved at its start. No VoiceOver or OS preferences were changed.
These checks do not establish Linux GUI, screen-reader or performance acceptance.

## Retained failures and limits

The initial Signal Studio extraction used the reserved OCaml keyword `effect`
as a parameter; the compiler rejected it. Renaming it to `ui_effect` fixes the
parse error, and the complete build/format/test command above subsequently passes.

An initial broad gallery `core` invocation was stopped by its scratch wrapper's
120-second deadline during shimmer validation. That run is incomplete. The
existing shell assertions were extracted into a separately selectable `shell`
section; `core` still calls them. No broad presentation-family acceptance is
claimed for this refactor. The initial broad run had no outer clipboard snapshot,
so full clipboard-representation restoration is not asserted for that run.

The first focused shell run expected the obsolete slider text `Level: 35`.
The unchanged numeric page renders `Value: 35 · committed: 35`. Matching the
current initial and incremented/committed labels makes the full shell sequence
pass; both failed and successful logs are retained.

[Source changes, exact commands, logs and native artifacts](example-readability-och17/validation.tar.gz)
and their [checksum manifest](example-readability-och17/manifest.json) preserve
the checkpoint. Hosted run 37387307992 covers earlier `f6e34e2`, not these changes.
Current-source hosted checks/review and the wider OCH-17/OCH-41 acceptance remain
open.
