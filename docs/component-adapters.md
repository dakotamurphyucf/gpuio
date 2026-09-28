# Writing and maintaining component adapters

An adapter translates a public OCaml contract into native GPUI behavior. It is
not a Rust object wrapper. Applications own their domain model and asynchronous
work; Rust owns focus, editing sessions, pointer capture, layout and paint.
Events return asynchronously through the bounded bridge. Never call OCaml from
a layout, paint, input delegate or other synchronous native callback.

Start with the [engineering standards](design/engineering-standards.md),
[catalog](catalog/README.md), and the relevant family's design and evidence.
For application-specific native components, prefer the existing
[extension SDK](design/extensions.md) and its independent consumer example.
Expose disabled/read-only state on the actual accessible controls inside a package;
state on the host group does not automatically propagate to child controls.
Keep live event guards in addition to render-time accessibility metadata.
Do not add an internal bridge opcode merely to avoid using that SDK.

## Choose the smallest implementation that owns the behavior

| Requirement | Implementation | Reference |
| --- | --- | --- |
| Labels, badges, cards, form layout, descriptions | Ordinary public View composition and semantic metadata | `lib/core/presentation.ml`, `lib/core/form.ml` |
| Native editing or focus/input state | Reuse pinned GPUI Base behavior with a GPUIO native adapter | `lib/eio/text_input.mli`, `rust/native/src/editor.rs`, `docs/design/native-editor.md` |
| Controlled choice or popup | Validated immutable configuration, queued proposals, explicit confirmation/cancel state | `lib/eio/date_picker.ml`, `lib/core/date_picker.mli` |
| Large collections/documents | Existing managed viewport and prepared immutable data contracts | `docs/design/managed-lists.md`, `docs/design/documents.md` |
| New application-specific native widget | Statically linked SDK registration and public codec/controller | `examples/extension_consumer`, `examples/signal_studio` |

Read the exact pinned sources before deciding what can be reused. Longbridge's
`gpui-base` and styled `component` crate have different responsibilities. GPUIO
vendors the former against one Zed GPUI package identity. The styled facade is
not a drop-in dependency: its manifest, theme globals, assets and composition
assumptions need a compatibility review. Matching an upstream widget name does
not establish behavior, accessibility or every styling option. GPUIO style tokens
and ordinary view composition are preferable when the upstream facade adds only
presentation. Preserve extracted algorithms' source attribution, as the chart
adapter does in `rust/plot/UPSTREAM.md`.

## Write the public contract first

Define a domain module with its principal `t`, an `.mli`, and a small intended
call-site example. Keep configuration, observations, proposals and commands
distinct. Constructors validate finite numbers, ranges, text bounds and unique
identities. State units explicitly: logical pixels, UTF-8 byte offsets, civil
dates, native revisions or durations. Keep wire versions separate from public
types and validate decoded values too.

For each adapter, specify:

- Who owns the value during interaction; whether an observation changes model
  state, a proposal requires acceptance, or an explicit command changes native
  state. Initial values apply once per native session, not on every render.
- Identity and lifetime on keyed reorder, configuration change, hide, unmount,
  remount and window close. Capture the live generation for commands and reject
  stale delivery; do not resolve an old command against a replacement by name.
- Ordering and coalescing. A latest-state reducer must recheck current disabled,
  read-only and revision constraints. Terminal command/submit events cannot be
  casually merged with ordinary observations.
- Keyboard defaults, focus restoration, IME interaction, accessibility role,
  name, value, state and actions. Do not require a late OCaml callback to cancel a
  default action that Rust has already performed.
- Theme, scale, reduced motion, clipping and hidden content policy. Paint-only
  animations stay native; visibility and teardown must stop unnecessary frames.
- Limits on live resources, bytes, outstanding requests, retained rows/history
  and caches. Define overflow, cancellation and typed recovery outcomes.

Application tasks and data must not accidentally die with a virtual row. Native
leases do die when their placement unmounts. Bonsai branch model retention is a
separate choice: Component Studio retains sample values, remounts native editors
from their initial value, and clears transient chooser/toast state on departure.

## Implement and prove the boundary

Add Core types, Bonsai views and Eio controllers only where each layer is needed.
Keep first-party I/O in Eio scopes. A controller borrows its owning window; it
must not keep disposed windows alive. Contain exceptions/panics at the FFI
boundary. Pair protocol changes with independent OCaml/Rust fixtures; round trips
alone can conceal the same encoding error on both sides.

Use meaningful expect tests for model transitions, malformed input and delayed
events. Native tests should exercise the actual behavior being claimed: real OS
keyboard/clipboard/IME is distinct from GPUI-injected events or an AX action.
Check native accessibility, multiple windows, removal with pending work, and
bounded repeated lifetimes. A screenshot complements those assertions.

Add a public gallery preview and an installed-library consumer where applicable.
Record commands, checked revision, OS/architecture, display and actual outcomes
in `docs/evidence` and Linear. Update `docs/catalog/families.json` and the detailed
style/event ledger; a helper maps to its owning API rather than becoming a fake
standalone widget. Build success cannot promote a pending row to validated.
The current release gate is macOS; Linux build/unit/private-bus/consumer checks
remain required, with GUI qualification tracked separately in OCH-47.

## Reconstruct before upgrading

Normal builds use committed vendor sources and lockfiles. Reconstruction must
reproduce those inputs without editing a working switch or vendor directory.
`third_party/sources.json` records archive, patch and source hashes. The scripts
refuse an existing output rather than modifying it in place.

For GPUI Base, choose a new destination under ignored scratch:

```sh
python3 scripts/vendor_gpui_base.py --output scratch/reconstruction/gpui-base
diff -ru vendor/gpui-base scratch/reconstruction/gpui-base
```

This command needs Python 3.12+, curl and patch; it verifies both the downloaded
archive and the patch, retains the upstream manifest/license, and unifies GPUI,
macros and sum-tree with the recorded Zed revision. `--archive PATH` accepts a
local archive with the same hash validation. A nonempty diff needs investigation.
Use a fresh output path when repeating the check.

The Bonsai reconstruction script writes a whole `vendor/` directory and has no
output option. Run it in a disposable staging tree containing copies of
`scripts/vendor_bonsai.py`, `third_party/sources.json` and
`third_party/patches/`, preserving that layout. Compare each reconstructed
`native_bonsai` member with its committed vendor directory. Do not move/delete
the live checkout's vendor tree to make the script run. Ordinary reconstruction
omits `--record-archives`; that flag records new hashes and belongs only in a
deliberate source-update review. See [development](development.md) for opam and
Cargo lock updates in the isolated environment.

An upgrade changes exact pins, patch files, their hashes, lockfiles and source
snapshots together. Review removed patches as carefully as new ones, including
grapheme editing, document selection, lifecycle snapshots, retained action paths
and accessibility changes. Do not assume an upstream replacement preserves the
GPUIO contract. Re-run cross-language fixtures, lifecycle and native affected
families, installed consumers and both platform build suites. Check for duplicate
GPUI package identities in the resolved Cargo dependency graph.

Catalog snapshots have their own exact-Git-blob/SHA-256 manifest under
`docs/catalog/sources`. On an intentional catalog pin change, regenerate the
snapshots from that commit, update their manifest, then run
`python3 scripts/audit_component_catalog.py --write` and review all new, removed
and changed nested exports and values. That command updates the inventory only;
it does not bless family behavior or dependency compatibility.

Preserve upstream copyright and license files in all copied sources. Update
[THIRD_PARTY.md](../THIRD_PARTY.md) and distribution acknowledgements, including
syntax/theme asset notices and extracted chart sources. A project's Apache-2.0
license does not replace the licenses of its dependencies.
