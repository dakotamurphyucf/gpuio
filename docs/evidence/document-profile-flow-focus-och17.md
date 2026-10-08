# Flow document plugin focus clipping — OCH-17

2026-10-08, local macOS 14.5 arm64, milestone-07-gallery-release based on
b979b921. This fixes a production keyboard-navigation defect found while
qualifying the public native document profile. OCH-17 and OCH-41 remain open.

The gallery uses `Layout.Flow` for its review profile, which embeds an independent
150-pixel scrolling viewport. After “Show review start”, the “Open scroll review”
button is below that viewport. Tab from “Show review end” nevertheless focused
that invisible button. Native geometry confirmed the button at y1089–1115 while
the viewport occupied y805–955, unchanged before and after focus/Tab.

`TextView::paint` enabled native input clipping only for a preview line limit or
its own scrolling layout. A flow document with a plugin-owned viewport satisfied
neither condition. Child Divs therefore admitted clipped controls to the tab
order. All document child painting now uses `with_clipped_input`; native content
masks determine eligibility, while the plugin continues to own its scroll offset.
This neither adds automatic inner scrolling nor changes the OCaml API.

The old native test also read focus immediately after asynchronously posting Tab
and could accept an `AXWindow` as a successful exit. Hosted run 37772649589,
macOS job 113295747167, exposed the stale-origin read after the earlier platform
focus forwarding fix. A bounded acknowledgement loop now requires actual focused
state, rejects windows and the clipped button, and never resends Tab. With this
stronger oracle the production defect reproduced locally. Earlier scroll-profile
results do not establish this particular Tab-exit assertion; these results
supersede that part of the [older evidence](document-profile-scroll-och41.md).

Validation:

- Added `flow_document_respects_plugin_owned_scroll_clipping` alongside the
  existing viewport test. Before repair it failed: actual “Profile scroll bottom”
  versus expected “Collapse”. After repair both layouts pass the same traversal,
  reverse-entry, wheel reveal and activation checks.
- Full native image-feature unit suite: **1,187 passed, two existing ignored**.
  Strict all-target Clippy and workspace formatting pass.
- Actual macOS gallery walkthrough: **11 checks pass at Comfortable and Large**,
  including code/table actions, inline/block Tab, property changes, profile
  remount, keyboard and wheel reveal, scroll retention/reset and source cleanup.
  Each uses foreground OS key delivery. Tab exits to the next “Inherit” button,
  observed after 33.4/31.3ms; these samples are test acknowledgements, not latency
  benchmarks. Both child windows close and processes are reaped. Clipboard state
  is preserved by the wrapper. No VoiceOver or OS preference changes.
- Three portable focus-wait tests pass: delayed acknowledgement, forbidden
  clipped target and stuck/window/unfocused/missing target rejection. Added to CI.
- Updated GPUI Base patch and manifest hash. Reconstruction from the pinned
  hash-verified archive matches all **244 tracked files**. Example inventory
  (432 sources / 268 reviewed groups), Ruff, actionlint and diff checks pass.

Commands (repository isolated environment; jobs2):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -p gpuio-native --lib --features native-image-tests --locked -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -p gpuio-native --all-targets --features native-image-tests --locked -j 2 -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
python3 scripts/test_macos_document_profile.py --output scratch/profile-flow-normal
python3 scripts/test_macos_document_profile.py --large-density --output scratch/profile-flow-large
python3 scripts/test_document_profile_focus_wait.py
```

Actual walkthrough output directories are `profile-flow-native-003` and
`profile-flow-native-large-004` under the root agent's scratch workspace. They
were invoked through `mac_clipboard.preserved_clipboard`. After those runs a
precondition assertion was added requiring the recorded pre-Tab visibility to be
false; both archived reports already record that value. No renderer change
followed those runs.

Gallery SHA256 after repair:
`e86e29605b7c64f37887ab116439c7001f9d14594b90ce7949b1ac345513a7fc`.
[Archive manifest](document-profile-flow-focus-och17/manifest.json) identifies
21 artifacts with SHA256 and sizes: before/after reports, screenshots,
walkthroughs, application logs, failing regression, passing suite/Clippy,
vendor reconstruction and the hosted failure excerpt.

This is local macOS and TestPlatform evidence. It does not qualify VoiceOver,
Linux desktop behavior, arbitrary plugin composites or all release gates.
It does not explain the separately reported benchmark row overlap. The ongoing
hosted run predates this repair; a later batch must verify corrected CI behavior.
