# Flat split-group public bridge and gallery — OCH-41

Local checkpoint, 2026-10-03, macOS development checkout, base `83eb87e` plus
the uncommitted milestone tree. Builds use the repository's isolated stock
OCaml 5.3/Bonsai v0.17 environment and pinned Rust dependencies. This extends the
[native widget evidence](split-group-widget-och41.md) and
[accepted contract](../design/split-group.md); it does not complete milestone 07.

## Implemented behavior

Core and Bonsai `View.split_group` now mount up to 64 stable-ID panels, including
empty/single-panel groups, per-panel bounds and visibility, both axes, reset
generations and one-shot serialled resize requests. Shared/per-handle appearance
and optional passive grips preserve native input ownership. The Navigation
gallery's **A workspace that adapts to you** uses documented public APIs and a
retained Eio text input; controls reorder panels, hide the inspector, add/remove
an outline, switch axes, constrain sizes, reset, request resize and show grips.

Kind56, Op104 and Event73 are paired in both languages. Tree admission validates
the exact structural content/grip slots, passive descendants (including later
edits), appearance budgets, generations and retained memory. Core moves panel
wrappers by ID. Native callbacks check current configuration, handler, revision,
request serial and snapshot before enqueueing; dispatch uses the current OCaml
closure. No layout or paint callback invokes OCaml synchronously.

The production Host retains child editors, controls and resources across reorder
and hide/show. Each native pane clips input and keyboard traversal to its actual
rectangle; wholly clipped panes have an identified hidden accessibility ancestor.
Removed/hidden handles and offscreen focused children lose focus. Explicit frame
and close hooks cancel gestures and retire owners without an idle frame loop.

## Local validation

Commands prefixed with `GPUIO_JOBS=2 ./scripts/gpuio exec`:

- `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests --lib split_group_host_test`: **3 passed**. Production native event dispatch checks keyboard resize observations, retained control/handle identity, intra-pane focus clipping, hidden accessibility ancestry and blocked late actions. A retained editor preserves text, selection and focus through reorder, rejects typing while hidden, survives resize and releases on removal.
- `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests --test split_group_admission --test split_group_widget`: **11 passed** (three admission/replay, eight native widget).
- `cargo test --offline --locked -j2 -p gpuio-protocol`: **350 passed, no skips**.
- `dune build -j2 @runtest @fmt examples/gallery/main.exe`: **pass**.
- `cargo test --offline --locked -j2 -p gpuio-native --features native-image-tests,native-canvas-tests --lib`: **728 passed, two existing private-D-Bus skips**.
- `cargo clippy --offline --locked -j2 -p gpuio-native --all-targets --features native-image-tests,native-canvas-tests -- -D warnings`: **pass**.

The reviewed `split-group-public.hex` fixture contains six transactions produced
by the public Core reconciler: mount, reorder, hide, reset/request, clear request
and disposal. OCaml checks exact bytes; native admission and production Host
replay them. The Host emits exactly one request result (`c=100, a=130, b=70`),
preserves child identity and releases the removed group/control. Separately
hand-assembled config/appearance/snapshot/operation/event fixtures check paired
schema independently of the public transaction generator.

Regression testing found and fixed three integration faults: the host fallback
focus check omitted group handles; adding another large recursive render branch
overflowed the ordinary test stack (fixed by factoring managed-child dispatch,
without enlarging the stack); type-erasing a clipped pane dropped its outer
accessibility identity (fixed with an explicit stable semantic identity).

`GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery
--workspace scratch/agents/root-20260929-m7-resumed/split-group-installed-gallery`
passes from freshly staged installed packages (`run=False`), including the new
gallery module. It does not install into or mutate an opam switch.
The catalog structural audit and `git diff --check` pass; these do not establish
behavior or release acceptance. Scratch logs use the `split-group-` prefix in
`scratch/agents/root-20260929-m7-resumed/`.

## Physical walkthrough still required

Open the public gallery's Navigation page and reveal **A workspace that adapts
to you**. Edit the draft, then reorder/hide/show/insert/remove panels and verify
the draft and caret survive. Drag each divider beyond its handle and release;
cancel a second drag with Escape. Focus **Resize Files and Draft** and use
horizontal arrows/Home/End, then switch axes and use vertical arrows. Check
VoiceOver names, ranges and increment/decrement. Exercise the 200px limits,
320px request, reset and custom grips in both themes/scales. Verify final-size
observations, clipping in a narrow window and quiet idle/teardown behavior.
Repeat under actual GPU rendering and include release resource measurements.

The tests above use **TestPlatform**, not operating-system windows. They do not
establish physical macOS keyboard/IME/VoiceOver/GPU acceptance. No Linux desktop,
hosted CI, release publication or Linear-write result is claimed. OCH-41 catalog
qualification and OCH-17 macOS/resource/distribution/release gates remain open;
required Linux automation remains separate from deferred OCH-47 desktop checks.
