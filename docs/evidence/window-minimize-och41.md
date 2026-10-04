# Window minimize and source review — OCH-41

2026-10-04, local macOS arm64 worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. Physical minimization/restoration has
not been run. This does not complete window-family or milestone acceptance.

`Window.Command.Minimize` now travels through the existing correlated Eio/native
window request lane and calls GPUI's platform minimization on the native thread.
It does not enter close/quit routing, cancel the window scope or replace the
native tree. The snapshot response retains its existing fields; there is no
new minimized-state observation or OS-transition completion guarantee.

The unpublished epoch-3 window command adds **tag 8**. Existing command tags and
snapshot layouts remain unchanged. The independent OCaml/Rust message fixture is
`0b07000108` for correlation 7, window slot 0/generation 1. The Runtime gallery adds
an explicit **Minimize this window** button through the public API.

The [source review](../catalog/window-review.md) covers pinned `TitleBar`,
`WindowBorder` and `WindowExt`. The latter's exact source is now retained in the
catalog manifest, extracted after validating the recorded GPUI Kit archive hash.
It reveals remaining custom-chrome and native gesture/frame work; ordinary
standard windows do not establish those functional equivalents.

## Local checks

- `cargo test -p gpuio-protocol --test window --offline --locked -j 2` in the
  repository environment: **3 passed**, including the new independent command
  fixture and the existing command/observation checks.
- `dune build -j 2 @runtest examples/gallery/main.exe` in the repository
  environment: passes, including the OCaml fixture and public gallery build.
- `python3 scripts/audit_component_catalog.py`: passes, with 146 module entries
  mapped to 43 families. This checks structural coverage and source hashes only.

The full native library suite with `native-image-tests,native-canvas-tests`
passes **877 tests**, with two existing macOS private-bus skips. Strict all-target
native/protocol Clippy (`-D warnings`), repository formatting and whitespace checks
pass. These regression checks do not invoke the new OS minimize operation.
No hosted check of these changes is claimed.

No OS windows were opened. The pinned GPUI TestWindow's `minimize` method is
unimplemented, so these checks do **not** exercise physical minimization or claim
to simulate it. Before accepting this feature, minimize from the gallery, restore
through the platform, verify the same window/editor drafts and task scopes remain,
and exercise close/quit and repeated operations. Run current required Linux
nongraphical checks; actual Linux desktop behavior remains OCH-47.

## Handoff repair

`docs/status.md` now lists current milestone gates instead of interleaving current
and obsolete implementation states. The preceding 2,834-line document is preserved
byte-for-byte as `docs/status-history.md`, in the same directory so relative links
remain valid. Its SHA-256 is
`39c9d05fcad86fb9311d9b653ff6fcdd07d15d51d7abbe5e046fa682f03c4d4c`.
The catalog summary's obsolete Form/plain-input/custom-step implementation claims
are corrected against current interfaces and feature evidence. No historical
test result or open acceptance gate was converted into a completion claim.
