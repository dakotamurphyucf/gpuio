# GPUIO macOS expanded-state and outline-row adaptation

This is the published `accesskit_macos` **0.26.3**, at AccessKit revision
`c88605b96d04431f9c3c792464a0f2f253480e94`, with two small patches.
The upstream MIT/Apache-2.0 notices and both license texts are preserved. Source,
archive checksum and original per-file checksums are recorded in `UPSTREAM.json`.
Cargo uses the registry-normalized manifest, retaining its exact dependency ranges.
No GPUI/AccessKit version upgrade is involved.

The adapter's original `PlatformNode` did not implement `isAccessibilityExpanded`
or allow that selector, even for AccessKit nodes with `expanded = Some(true)`.
Consequently a GPUI button carrying `aria_expanded(true)` reported false through
AppKit. This was reproduced by GPUIO's actual native disclosure test.

`expanded-state.patch` adds a read-only expanded getter, advertises the selector
only for nodes that carry expanded state, and posts a value-changed notification
when expansion changes independently of the node's ordinary value. Activation
continues through existing accessibility Press and the asynchronous application
intent bridge. It does not add a second state owner or synthesize toggle values.

`tree-state.patch` adds the outline-row `isAccessibilityDisclosed` getter and
`accessibilityDisclosureLevel`, with selector availability restricted to TreeItem
nodes carrying the respective metadata. AccessKit's one-based level becomes the
outline row's zero-based indentation. The native GPUIO tree test reproduced a
child row reporting indentation zero and no disclosure getter before this patch.
There is no dependency/version change or second expansion owner.

Apple documents [outline-row disclosure](https://developer.apple.com/documentation/appkit/nsaccessibilityprotocol/setaccessibilitydisclosed(_:))
and [disclosure level as row indentation](https://developer.apple.com/documentation/appkit/nsaccessibilitydisclosurelevelattribute).
The existing generic expanded getter remains for disclosure buttons; leaves do
not advertise a disclosure state. Notifications continue through the existing
expanded-state value-change path. No VoiceOver speech or external AX notification
observer acceptance is claimed by these getter tests.

Reconstruction: download the crate archive from `UPSTREAM.json`, verify its SHA256,
extract `Cargo.toml`, `Cargo.toml.orig`, README/CHANGELOG and `src/`, then apply
`patch -p1 < expanded-state.patch` and then `patch -p1 < tree-state.patch` inside
that directory. Fetch LICENSE-APACHE and
LICENSE-MIT from the pinned upstream Git revision and verify their recorded hashes.
`UPSTREAM.json`, this note and the patches are GPUIO provenance additions. The
registry archive's Cargo.lock and Cargo cache metadata are not build inputs.

The root workspace and generated static extension backend both carry the same
`[patch.crates-io]` path. Dune explicitly tracks this source tree. Custom backend
workspaces must propagate that patch: dependency manifests cannot impose a Cargo
patch on their consumers. `scripts/compose_backend.py` does this automatically.

Validation lives in `rust/native/src/navigation_test.rs` and `tree_view_test.rs`:
the tests query actual AppKit expanded/outline state before and after collapse;
the navigation test also invokes native Press. The
standard native controls/tabs regressions continue to cover other adapter roles.
No VoiceOver speech or external AX notification-observer acceptance is claimed by
the getter test. Remove this fork when an evaluated pinned upstream adapter
provides the same behavior and the native regression passes without it.
