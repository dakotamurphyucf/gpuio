# Assets and clipboard: pinned behavior review

OCH-41 review, 2026-10-04. Exact Longbridge GPUI Kit revision
[`84f57fdfcb4910623fb0bb7f795b077e249f9271`](https://github.com/longbridge/gpui-kit/tree/84f57fdfcb4910623fb0bb7f795b077e249f9271)
inputs are retained in `sources/manifest.json`: `component-icon.rs.txt` and
`component-clipboard.rs.txt`. Downloaded bytes match Git blob identities
`4a66e5bdc516ce25e4942776a2ddc7d8da3b25b5` and
`006079cd6e038910a48cd49e9014c01e8191dfdf`, respectively; SHA-256 hashes are in
the manifest. These are reviewed source inputs, not new compiled dependencies.

## Icon and asset mapping

| Pinned behavior | GPUIO mapping and boundary |
| --- | --- |
| Named bundled icon or application asset path | Applications obtain bytes with explicit Eio capabilities and register `Asset.Source`. A name-to-source module can wrap that acquisition. GPUIO does not implicitly read paths or require the upstream Lucide catalog/macro. Bundled artwork still needs its own license review. |
| Owned SVG bytes, shared on clone | `Asset.Source.of_bytes ~format:Svg`, scoped `Gpuio_eio.Asset.register` and borrowed handles. Native image leases outlive retirement only for existing readers; a handle does not extend registration lifetime. Replacing the configured asset is explicit. |
| Monochrome tint, explicit/fallback foreground | `View.icon` tints the decoded SVG alpha mask using native inherited foreground/theme/state styles. `View.image` instead preserves source colors. This is not a promise that every upstream SVG rendering choice matches pixels. |
| Size overrides and small/medium/large presets | Width/height styles; `Icon.Decoration` defaults to 16 logical pixels and permits explicit styles. Application appearance presets can provide other sizes. No automatic text-size-dependent icon sizing is currently documented. |
| Empty icon | Omit the child or render an empty layout slot with an explicit size. Empty encoded asset data is intentionally invalid. |
| One-shot and retained Rust view construction | Pure OCaml descriptions and stable keyed native ownership; applications do not need Rust entity handles. |
| Arbitrary SVG transformation and rotation, latest transform wins | No direct public `Icon.Config` transform currently exists. `Animation` does not expose rotation. Canvas shape transforms or a statically linked extension are separate APIs; they do not establish parity for a rotated icon/control slot. This remains an explicit surface gap requiring an implementation or an accepted scope decision before broad icon parity is claimed. |
| Semantic image versus control decoration | GPUIO requires meaningful/decorative descriptions. An icon-button owner supplies its name/action; artwork is decorative. This is a deliberate explicit accessibility contract. |

Existing [asset evidence](../evidence/assets-och11.md),
[button-icon evidence](../evidence/button-icons-och11.md), image-mask native tests
and the public Assets gallery cover scoped sources, decode, tint and ownership.
This review does not replace current physical pixels/input/accessibility or
independent-consumer checks.

## Standalone clipboard component

The pinned `Clipboard` is a ghost, extra-small copy/check button. It accepts a
literal string or evaluates a Rust value function at activation, writes a string
through GPUI, invokes an optional copied callback and shows a check for two
seconds. During that feedback interval it does not install its copy handler.
An optional tooltip and keyed native feedback state complete the component.
The source does not define rich clipboard formats, reading clipboard contents,
a durable OS write acknowledgement or a synchronous foreign-runtime callback.

Current GPUIO editor, document and table copy paths use their native selection
owners. They do **not** provide the independent public operation needed for a
button that copies arbitrary application text. Window `selected_text` explicitly
reads registered read-only selection; it is neither clipboard access nor an
editable-value read. The family remains incomplete rather than being classified
as covered by those existing paths.

The intended minimal adapter is an explicit typed asynchronous native plain-text
write, bounded and validated at both sides, with a result describing the actual
platform guarantee. A public button composition can capture the current OCaml
value when its activation effect runs, await native command processing, then
apply copied feedback through a scoped Bonsai timer. That ordering differs from
a Rust closure evaluated synchronously during native click dispatch and must be
documented. No synchronous call into OCaml should be introduced. Feedback must
retire on unmount/window close and reject stale completions after replacement.
The timeout must not require a permanent per-component timer.

Before acceptance: public literal/current-value examples; UTF-8/byte bounds and
codec/rollback tests; command cancellation/backpressure and stale generation
checks; real macOS clipboard readback with original contents restored; ordinary
keyboard/AX activation; tooltip/feedback timing and independent-window cleanup.
Linux compilation and pure/private-bus/consumer checks remain required; actual
Wayland/X11 clipboard qualification belongs to OCH-47. This is an implementation
plan and gap record, not shipped functionality or completion of OCH-41.
