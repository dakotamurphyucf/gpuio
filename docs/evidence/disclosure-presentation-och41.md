# Rich disclosure presentation evidence — OCH-41

2026-10-02, local macOS dirty worktree based on `83eb87e`. No OS windows were
opened. This records implemented rich headers, not measured reveal or release
completion.

Three added Core expect tests pass for heading metadata, one native trigger per
item, retained identities across title/expansion updates, queued presses after
disable, unknown/duplicate/interactive labels, aggregate label limits, heading
levels and Unmount skipping collapsed content construction. A fourth test compares
the exact public initial/collapse/reopen transactions with checked-in fixtures.
Those fixture assertions also pass in the full-suite checkpoint.

The native TestPlatform test decodes the actual public transaction sequence,
mounts its host and verifies:

- Three headings and three native buttons, expanded state and disabled metadata.
- Arrow navigation skips the disabled middle item; Home returns to the first.
- Enter emits exactly one toggle intent.
- Actual native editor input changes the draft.
- Collapsing returns focus to the header and removes the editor from the AX tree.
- Retain keeps the same editor focus handle and draft through collapse/reopen.

`cargo test -j2 -p gpuio-native --features native-image-tests --lib
disclosure_view_test --offline` passes (one test).
`dune build -j2 @test/view_api/runtest` passed the three behavioral tests before
fixture generation. `dune build -j2 @check examples/gallery/main.exe` also passes.
Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec`. Full OCaml tests, formatting and the gallery build pass. The full native library
suite passes **598 tests with two existing skips**; strict all-target Clippy for
native/protocol passes with warnings denied. Catalog source audit and diff
whitespace checks pass. Exact broad commands:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --lib --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
```

The gallery demonstrates optional-single, required-single and multiple expansion,
global/per-item disabling, rich titles and Retain/Unmount with a native notes
field. This build is not a physical appearance or accessibility qualification.
The [contract](../design/disclosure-presentation.md) and
[source review](../catalog/disclosure-review.md) explicitly track measured reveal
as remaining work. Physical macOS/VoiceOver, measured resources, installed
consumer and platform/release gates remain open.

## Reveal layout checkpoint

The native motion/layout code is independently exercised but has no public host
connection yet. Nine motion-state tests and five TestPlatform layout tests cover
spring reversal and deadlines, stale/closed paint ordering, exactly one final
settled-layout request, first-opening measurement behind a zero clip, wrapping,
padding/borders, absolute height constraints, parent-dependent fallback and style
changes during motion. Settled resize uses current natural layout without a
cached-height correction frame. External margins/gaps are deliberately not
interpolated; the design documents this limitation.

The same native library command above passes **612 tests, zero failures and two
existing skips** after these additions. The same strict native/protocol Clippy
command passes. Rust formatting and diff whitespace checks pass. No OCaml/wire
code changed in this checkpoint; the preceding full Dune/gallery checkpoint
remains the OCaml evidence. No OS windows were opened, and these results do not
qualify physical GPU output, keyboard/IME, VoiceOver or release performance.


## Integrated reveal — 2026-10-02

The public Core/Bonsai API, bridge and native host now connect measured reveal.
The Navigation gallery opts into `Disclosure.Motion.standard` and provides an
Animate toggle. Immediate remains the API default. Three Core expect tests cover
paired wire bytes, stable panel identity, optional configuration reset, immediate
Unmount descendant removal/recreation, and forwarding through every helper.

Two native host tests run on GPUI TestPlatform with its deterministic clock.
The retained-editor test composes the existing public rich-header transactions
with the new reveal operation (whose bytes have independent Rust/OCaml fixtures).
It verifies actual partial height, reversal from that painted position, retained
editor identity/draft, focus return, hidden AX ancestry, and blocked queued AX
Focus, pointer and keyboard attempts while closing. The lifecycle test verifies
immediate Unmount disposal, opening newly mounted content from zero, reduced
motion, deactivation, full viewport clipping, configuration removal and window
close cleanup. Diagnostic references deliberately retain the old state to prove
cleanup cancels its pending lease rather than relying only on deallocation.

A failed raw AX-node-absence assertion was corrected to test the relevant
contract: outgoing visuals keep raw nodes under a hidden accessibility ancestor,
while native actions remain blocked. This does not substitute for physical
VoiceOver acceptance. A separate admission test found and fixed a real fast-path
issue: consistency checks now run for every changed node, including style-only
transactions, rather than only structural edits. Invalid transactions roll back;
accepted configuration reserves and releases 512 owner bytes plus config storage.

Current checks use the repository's isolated environment with two build jobs:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --lib --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native -p gpuio-protocol \
  --features native-image-tests --test reveal --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-protocol --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test -j2 -p gpuio-native \
  --features native-image-tests --test tree --test session --test disclosure \
  --test navigation_stack --test allocation --offline
GPUIO_JOBS=2 ./scripts/gpuio exec cargo clippy -j2 -p gpuio-native -p gpuio-protocol \
  --all-targets --features native-image-tests --offline -- -D warnings
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @runtest @fmt examples/gallery/main.exe
```

The full native library suite passes **614 tests, with two existing isolated
D-Bus skips**. All **319 protocol tests**, the new admission test and **22 existing
tree/session/disclosure/navigation/allocation regressions** pass. Strict lint,
full OCaml tests, formatting and gallery build pass. The renderer borrows the
parent style while constructing children, avoiding a style clone for every
ordinary node. The full native suite also passes after that change; current build evidence
is recorded alongside the exact commands above.

No OS windows were opened. Layout and lifecycle checks are TestPlatform evidence;
physical macOS rendering/input/IME/VoiceOver, sustained streaming/typing and
resource measurements, installed-consumer distribution, current required Linux
checks and release gates remain open.
