# Color palette sections, panels and appearance

OCH-41, 2026-10-02. These APIs extend the existing Rust-owned color input used by
both `Gpuio_eio.Color_input` and `Gpuio_eio.Color_picker`.

`Color_input.Config.create` accepts either the existing flat `palette` or
`palette_sections`. A `Palette_section.featured` or `Palette_section.group` owns
a nonempty list of labeled entries and a section label. Labels are nonblank UTF-8
without NUL, at most 256 bytes. A configuration has at most 32 sections and 256
entries total; a featured section may appear only once, first. Duplicate colors
remain valid distinct slots. Section order and entry order determine native
focus traversal; native IDs are never derived from color values.

Sections group the same native swatches. A featured row uses larger swatches;
ordinary sections have headings and wrapping rows. Applications supply localized
labels and their colors. The gallery demonstrates a featured row and nine color
families; it does not introduce a theme-global hardcoded application palette.

`Color_input.Appearance.create` controls swatch and featured size, swatch and
section gaps, corner radius, outline width, channel height, control gap, padding,
and selected/focused and hovered border colors. Sizes and channel height are
16..128 points; gaps/padding are 0..64; radius/outline are at most half the smaller
swatch size. All geometry is finite. Colors resolve through the application theme.
Unset border colors inherit foreground; outer style still supplies typography
and container styling. Core/Eio color views and both plain/rich color-picker
views accept `?appearance`. The picker applies it to its native draft, independently
of trigger styling.

Changing section metadata or geometry keeps the same native input, five text
editors, palette focus handles, model revision and active text draft/composition.
Changing panel visibility has the settlement rules below.
Palette entry or policy changes continue to use the existing color configuration
contract. Existing captured gestures still cancel when their track geometry moves;
appearance is not an exemption from pointer-capture safety. Transient hover preview
clears when presentation changes. Regrouping never confirms a popup; explicit
Apply remains required. Native section labels expose Group semantics, while each
swatch retains its label, radio/selected state and input gates.

Bridge Op87 `Set_color_presentation` carries an optional presentation containing
section metadata, resolved appearance and panel configuration. None means flat
layout, both panels and default geometry.
Native color configuration, snapshots and commands retain their previous bytes.
The Core configuration flattens sections to the existing palette and submits
metadata separately. Reconciliation compares color policy separately from layout,
so a layout-only update sends no redundant color configuration operation.

Decoding bounds strings, vector lengths, counts, finite geometry and packed
colors. Final transaction validation checks that nonempty section counts cover
the palette exactly; combined palette/presentation changes work in either order
and mismatches reject atomically. Retained accounting includes section structures
and section/tab label storage. Native rendering performs bounded work over the accepted
entries; no layout/paint callback into OCaml, polling timer or extra editor owner.
Transparency uses the existing fixed sixteen checkerboard quads, each clipped
from the complete rounded swatch so large radii also clip neighboring tiles.

## Palette/HSLA tabs

`Color_input.Appearance.create ~panels` accepts `Panels.all` (the default) or
`Panels.tabs ~palette_label ~channels_label ?initial ()`. Labels are nonblank
UTF-8 without NUL, at most 256 bytes each. `Panel.Palette` is the initial default;
`Panel.Channels` can select the channel panel instead. Initial selection applies
on mount and on a change from All to Tabs. Later label/theme/geometry/initial
updates retain the user's selected tab; initial is not a controlled value.

One native model and the same five editor entities serve both presentations.
Hex and Clear stay visible. Only the active panel paints and contributes input
or accessibility targets. The tab list has one selected tab stop; Left/Right
wrap, Home selects Palette and End selects Channels. Mouse and AccessKit Click
or Focus activate a tab. Hidden channel commands and stale palette callbacks
cannot edit the model. Read-only allows tab browsing; disabled, inert and modal
restrictions apply. Pointer-events restrictions do not disable keyboard or AX
activation. Tabs expose TabList/Tab/TabPanel roles and selected state.

User activation settles pending visible editor changes using existing blur
policy: valid text commits to the native draft; invalid or composing text
cancels. This is checked after GPUI mouse-down may have moved focus, so delayed
blur observation cannot strand an edit. AX Click has a direct native handler to
avoid synthesizing a pointer release into an outstanding drag. Switching away
from channels cancels capture, releases its hitbox and clears hidden track hit
boxes. A presentation-only update preserves visible hex composition, but hiding
a channel settles that channel's pending native text before disabling it.

`Color_input.focus` on an eligible channel reveals Channels and focuses the same
editor. Policy is checked first: for example, rejected opaque-alpha focus does
not switch tabs. Explicit focus cancels a captured drag. Focus in hidden content
moves to the selected tab; removing tabs while a tab is focused moves to Hex.
Owner removal releases the tabs and editors, and stale handler generations are
rejected. Tab changes do not confirm an application-controlled popup; Apply
remains explicit. The public gallery uses localized tabs for inline and popup
examples.

Op87 appends panel configuration after `hover_border`: tag 0 is All; tag 1 is
Tabs followed by palette label, channel label and initial panel (0 Palette,
1 Channels). The bounded Rust decoder rejects other tags and invalid labels.
This changes the new, unreleased Op87 payload; it requires matching Core/native
builds. Existing color policy, snapshot and command bytes are unchanged.

This contract does not imply OS
IME, VoiceOver, GPU or Linux desktop acceptance. See the
[source review](../catalog/calendar-color-review.md) and
[local evidence](../evidence/color-presentation-och41.md).
