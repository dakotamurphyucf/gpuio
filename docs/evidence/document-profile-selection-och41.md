# Custom document text selection — OCH-41

Local macOS arm64 checkpoint, 2026-10-04, on the dirty worktree based on
`83eb87e865c86717a8bc51b9db6fe1f379d909a9`. This uses the production document
host and GPUI TestPlatform, without OS windows. OCH-41/OCH-17 remain open.

## Reproduction and repair

A native pointer-drag test selected an inline plugin label successfully, then
returned empty text for the block plugin label `Profile card`. The block used
passive rendering with frame-local text state; its AST node never participated in
partial-selection lookup or clearing. Select-all and search had passed earlier
because they used separate paths. The failing regression is retained in
`document-profile-interactions-native-001.log`.

Declared block `Text` now uses the ordinary reader glyph-selection path. A parsed
occurrence owns its selection state, shared by rendering and the AST copy/clear
traversals. Parsed template occurrences receive fresh state, while redraws retain
it. The immutable displayed-text projection still supplies the same search glyphs.
No arbitrary custom renderer acquires implicit text-selection support.

In Markdown copy mode a fully selected projected block uses its declared Markdown;
a partial glyph range returns selected display text, because a plugin projection
has no character-level source mapping. Plain copy uses selected display text.
Select-all retains the existing declared copy-text/original-source contract.
See [the contract](../design/document-profiles.md#declared-block-text-selection).

## Local qualification

Tests cover forward/reverse selection of inline and block objects, redraw
retention, source replacement, Unicode substrings, whole/partial Markdown copy,
window selection clearing and independent sibling blocks. A separate stress case
traverses 96 passive plugin candidates before a real native control, in both
directions and out to the containing toolbar, without activating any control.
This establishes bounded functional traversal, not a physical frame-latency budget.

Logs live in ignored `scratch/agents/root-20261003-release-notices/`; they are not
build inputs. Commands use `GPUIO_JOBS=2 ./scripts/gpuio exec` unless shown otherwise.

| Command | Result | Log |
| --- | --- | --- |
| `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2 profiles::semantics::` | All four tests pass | `document-profile-interactions-native-004.log` |
| `cargo test -p gpuio-native --lib --features native-canvas-tests,native-image-tests --offline --locked -j2` | 918 passed; two existing macOS private-bus skips | `document-profile-interactions-full-001.log` |
| `cargo test -p gpuio-document-sdk --offline --locked -j2` | 15 passed | `document-profile-interactions-sdk-001.log` |
| `cargo clippy -p gpuio-native -p gpuio-protocol -p gpuio-document-sdk --all-targets --features gpuio-native/native-canvas-tests,gpuio-native/native-image-tests --offline --locked -j2 -- -D warnings` | Passed | `document-profile-interactions-clippy-002.log` |
| `GPUIO_JOBS=2 ./scripts/gpuio check-fmt` | Passed | `document-profile-interactions-format-check-001.log` |
| `dune build -j2 @runtest examples/gallery/main.exe` | Full OCaml suite and independent gallery backend passed | `document-profile-interactions-ocaml-001.log` |

Base text/selection tests: **238 passed**, `document-profile-interactions-base-001.log`:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec cargo test --manifest-path vendor/gpui-base/Cargo.toml --lib text --offline --locked -j2 \
  --config 'patch."https://github.com/zed-industries/zed.git".gpui.path="/Users/dakotamurphy/gpuio/vendor/gpui"' \
  --config 'patch.crates-io.accesskit_macos.path="/Users/dakotamurphy/gpuio/vendor/accesskit-macos"' \
  --config 'patch.crates-io.taffy.path="/Users/dakotamurphy/gpuio/vendor/taffy"' \
  --config 'profile.dev.debug=0' --config 'profile.dev.package."*".opt-level=1'
```

The initial Unicode test incorrectly treated a block's full AX layout width as
its glyph advance. Using a short logical-pixel drag within the glyphs fixes that
fixture; the source/copy assertions remain intact. Strict lint then removed one
redundant test closure. No test expectations were promoted. Catalog audit (146
modules / 43 families), seven edited-document relative-link checks and whitespace
checks pass. No OCaml expectation updates were needed.
The existing `block 0.1.6` future-compatibility warning remains.

## Maintained source

Four Base files changed: `text/{markdown_ext,displayed_text,node,inline}.rs`.
The pin remains `84f57fdfcb4910623fb0bb7f795b077e249f9271`. The maintained patch
SHA-256 is `d0d270991043787f9dbb800abd31bf28356b8c34608262858fd93565431e0971`.
Reconstruction reproduces all **235 source files** byte-for-byte, excluding only
the ignored local Cargo lock, target and Git directories.

```sh
python3 scripts/vendor_gpui_base.py \
  --archive scratch/agents/root-20260912-milestones/gpui-kit-84f57fd.tar.gz \
  --output scratch/agents/root-20261003-release-notices/document-profile-selection-base-reconstructed
```

The archive is hash-verified by the script; the scratch path is local evidence,
not a build dependency. Logs: `document-profile-selection-reconstruct-001.log`
and `document-profile-selection-reconstruct-compare-001.log`.

## Remaining acceptance

Physical keyboard/clipboard/VoiceOver, platform shaping, native plugin-owned nested
scrolling, performance budgets and the broader release gates remain open.
