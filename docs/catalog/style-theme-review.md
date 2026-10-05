# Style, sizing and theme helpers — OCH-41

Source review, 2026-10-04, against GPUI Kit
`84f57fdfcb4910623fb0bb7f795b077e249f9271`. The
[source manifest](sources/manifest.json) retains the four Base modules
`state_style`, `styled`, `theme`, `theme_tokens`; Component `sizing`, `styled`,
`theme/mod`; all six nested theme Rust modules; and the two embedded default
color/theme JSON files. These are original upstream bytes, not our maintained
Base fork. This review maps application functionality and records gaps; it does
not certify the entire gallery or every specialized control's appearance.

## Styling and sizing

| Pinned behavior | GPUIO mapping and important differences |
| --- | --- |
| Base `StateStyle`, instance refinement, active semantic states, disabled last | Typed `Style` and widget-specific appearance types. `Control_appearance` explicitly supports Base/Checked/Indeterminate/Disabled with disabled last. Generic Focused/Hovered/Pressed precedence and specialized permitted states are separate contracts; do not assume every state is supported on every native root. See [style](../../lib/core/style.mli) and [control appearance](../../lib/core/control_appearance.mli). |
| `h_flex` / `v_flex` | `View.row` / `View.column` set flex direction. **Base h_flex also centers the cross axis; GPUIO row does not.** Add `Align_items Center` when reproducing that layout. Columns retain stretching defaults. A full-height scrolling child still needs definite usable bounds. |
| Paddings, margins, corner radii, font-weight helpers and shadows | Checked per-edge/per-corner Style properties, shorthands, numeric font weights and `Shadow.create`. Application composition replaces Rust fluent traits. Property units, limits and last-declaration behavior are in the [numeric audit](../evidence/style-numeric-policies-och41.md). |
| `RoleOverride` and development debug borders/focus helpers | Native controls retain their semantic roles; public view semantics and passive artwork contracts control application accessibility. Explicit styles and structured native diagnostics cover debugging. Rust inspector reflection is development tooling, not another serialized widget. A generic arbitrary role override is not implied for every control. |
| Component `Size`, `Sizable`, `StyleSized` | Per-component checked size/appearance values, explicit Style dimensions and application scale models. The gallery's compact/comfortable/large scale demonstrates application logical sizing; it is not OS DPI emulation or a direct copy of upstream size presets. Table row height/cell padding, icon dimensions, input padding and typography remain individually configurable through their owning APIs. |
| Component themed focus ring, popover surface and rounded-full helper | Native focus styling and explicit border/shadow/radius composition. Upstream focus-ring artwork extends outside the border and can be clipped; its shadow blur values are already GPUI values, not CSS blur radii. GPUIO does not promise pixel identity with that preset. Global “square everything” is an application design-system policy: pass zero radii to the relevant component appearance values, rather than expecting `Theme.create` to rewrite geometry. |

Do not copy the upstream sizing helper names as mathematical guarantees:
`Size::min` picks the larger preset/value and `max` the smaller in the pinned
implementation and tests; custom `smaller` multiplies by 0.2. GPUIO's public size
constructors do not inherit those operations or string-to-Medium fallback.

## Theme ownership and values

| Pinned behavior | GPUIO mapping and limits |
| --- | --- |
| Base global Theme and semantic colors/radius/spacing/typography/shadow tokens | `Theme.create` is a validated **color** environment. Application records hold metric, typography and motion policy and construct typed component styles. Gallery `Palette` is an example. Native Base globals are implementation defaults, not mutable OCaml objects shared implicitly across windows. |
| Component Theme, legacy component tokens and Base projection | Public component-specific appearances and application composition. Upstream `change`/`sync_base` copy selected global fields into Base and install reader defaults; GPUIO resolves colors before submission and uses explicit document defaults. Mutating an application palette must flow through the relevant view/config descriptions. A color-only `set_theme` does not change every component's radius, motion, document syntax policy or font. |
| Semantic configuration merge | Upstream returns a complete semantic snapshot but projects only legacy-representable fields back into its global Theme. Spacing and detailed elevations do not all round-trip; invalid color strings can be ignored. GPUIO constructors return validation errors for invalid admitted values. There is no claim of importing that JSON schema byte-for-byte. |
| Ordinary/vibrant light/dark appearance | Implemented `Window.Appearance` and existing snapshot/on_change transport. The gallery offers Follow system and explicit palettes per window; [local evidence](../evidence/window-appearance-och41.md) covers routing, state retention and cleanup. The [physical macOS walkthrough](../evidence/window-appearance-och41.md#physical-macos-appearance--2026-10-05) also passes two-window OS switching, independent explicit palettes and retained native editing; automatic scheduling, high contrast and vibrant modes remain separate. |
| Scrollbar mode, states and motion | `Scrollbar.Mode`, `Appearance` and `Motion` map explicit policy, track/thumb states and finite transition times. They do not mutate Base's application-global theme. The asynchronous `Desktop.scrollbar_preference` snapshot exposes macOS auto-hide information; the pinned Linux getter is a fixed default and returns `Unsupported` through our API. The gallery explicitly applies a snapshot to its previews. Like upstream’s `sync_scrollbar_appearance` helper, this is an on-demand read, not automatic preference tracking. See the [contract](../design/scrollbar-preference.md). |
| Semantic motion durations, easing, springs and distances | Public `Animation`, native programs and component-specific Motion configurations. Applications can keep a shared policy record and apply it when building views; no second global Rust theme or per-frame OCaml callback is needed. Logical distances are explicit; there is no generic OCaml rem unit. Reduced-motion behavior has its own native contract. |
| Theme highlight style and built-in document defaults | `Document.Appearance`, `Document.Style`, explicit/default profiles and their native adapters. Highlight configuration is not automatically inherited from arbitrary named color tokens. See [document review](documents-review.md) and [defaults evidence](../evidence/document-defaults-och41.md). |

The source `ThemeConfig` contains metadata, optional text families/sizes,
radii/shadows, component colors and highlight configuration. Its separate
`SemanticThemeConfigFile` contains colors, metric scales, typography and shadows.
Those representations are deliberately different even upstream. Treating the
entire schema as a single FFI “theme object” would conceal ownership and validation.

## Colors and backgrounds

`Color` provides concrete RGBA values, named references and bounded opacity
composition; `Color_value.Rgba` provides strict ASCII hex parsing/formatting and
checked HSL conversion. Unlike the upstream helper's six/eight-digit `parse_hex`,
the public parser also accepts three/four digits. It rejects malformed input
instead of relying on permissive string parsing or native slicing.

The upstream Colorize trait includes HSL channel arithmetic, inversion, hue
replacement, HSL interpolation and alpha-premultiplied Oklab mixing. These are
pure palette-generation helpers, not native component state. GPUIO does not export
that trait or claim every transform has a convenience function. Applications can
calculate concrete values before constructing a Theme. `Background.linear_gradient_in`
does provide native two-stop sRGB/Oklab painting; it is not a general color-mixing
API or CSS parser. See [background](../../lib/core/background.mli) and
[concrete color](../../lib/core/color_value.mli).

Upstream named scales are embedded palette data, with separate hex/Tailwind-like
and two-stop gradient string parsers. `ThemeToken` stores both a representative
solid color and a renderable background. GPUIO keeps `Color.t` and `Background.t`
distinct; a gradient cannot masquerade as a foreground color. A two-stop gradient
can use named color stops, preserving dynamic palette resolution. The upstream
default palettes are retained as source evidence, not installed as required
application assets or promised GPUIO defaults.

## Registry, runtime files and font defaults

The Component registry loads named sets, keeps light/dark defaults, sorts defaults
before other themes and observes registry changes. Its directory watcher queues
filesystem notifications, rereads JSON and refreshes active themes. This is
runtime application configuration; it must not be silently classified as excluded
React hot reload.

GPUIO's corresponding ownership belongs to Core application state and scoped Eio
file I/O, followed by validated Theme/style/config construction on the UI domain.
Named collections and selection need no native registry or callback bridge.
The gallery now provides a [bounded file-theme example](../evidence/gallery-theme-files-och41.md)
with checked S-expression profiles, scoped Eio reads, recoverable errors, last-good
retention and stale-choice suppression. Reload is explicit. This is an application
format, not a built-in framework schema or a directory watcher. Local tests/build
pass; a physical picker/reload/editor walkthrough remains required.

The nested `mono_font` implementation prefers the platform default, then installed
monospace alternatives, then `.SystemUIFont`; explicit configured families are
left alone. GPUIO now selects the default once per application, preserving an explicit
configured family and preferring installed platform-specific monospace candidates.
Both rich code and source documents use the Base typography token. The Styles
preview uses the virtual system family. See the [default-font contract](../design/default-fonts.md);
[local evidence](../evidence/default-fonts-och41.md) covers policy/ownership, full
native regressions, the OCaml/gallery build and strict lint.

Do not infer an unconditional missing-font crash from the Component comments:
the pinned GPUI `TextSystem::resolve_font` tries a general fallback stack and
panics only when all choices fail. Its `all_font_names` also appends fallback
names, so presence in that list is not a universal proof that an arbitrary font
was installed. General fallback can choose proportional text, however; it does
not establish the intended monospace result. Default-font behavior and clean
machine font availability need concrete validation.

## Evidence and remaining work

The [appearance checkpoint](../evidence/window-appearance-och41.md) records nine
window codec tests and exact GPUI reconstruction. The subsequent [font checkpoint](../evidence/default-fonts-och41.md)
records 904 native tests, full OCaml/gallery and strict lint. Earlier color, gradients, specialized controls, scrollbar
and motion evidence remains scoped to its own contract. Hash-verified source
coverage and these passing tests do not certify all theme behavior.

Native scrollbar preference information now has an explicit snapshot contract
and a gallery action; no live subscription is claimed. Validation is recorded in
[the snapshot evidence](../evidence/scrollbar-preference-och41.md). The file-backed example is locally implemented with
explicit reload; automatic directory watching is not claimed. Default monospace
selection is implemented locally; clean-machine font qualification remains open. These are not new post-v1 deferrals. Physical
gallery review across themes/scales, focus/accessibility and final-source platform
checks remain release gates. Existing specialized appearance limits stay in their
family reviews rather than being hidden behind a global theme claim.


Native file-theme follow-up (2026-10-05): the [public gallery walkthrough](../evidence/gallery-theme-files-och41.md#physical-macos-walkthrough-and-cancellation-repair--2026-10-05)
now passes actual NSOpenPanel loading, same-appearance color reload, invalid-file
retention, native selection/undo, scale/window isolation and scoped delayed-read
cancellation. The app's Follow system choice wins over pending file work; actual
OS appearance switching now has its [own passing physical check](../evidence/window-appearance-och41.md#physical-macos-appearance--2026-10-05), including restoration of the original OS preferences. No watcher or upstream
JSON theme importer is implied.
