# Date and color picker ownership walkthrough

Read [pickers_page.ml](pickers_page.ml) and its [interface](pickers_page.mli).
[pages.ml](pages.ml) routes **Dates & colors** to `component window palette graph`. `B` aliases
`Bonsai.Cont`, `V` `Gpuio_bonsai.View`; Calendar/Date_picker/Color/Color_picker_controller alias
their `Gpuio_eio` adapters. Graph hosts reactive configuration/controller computations;
`let%arr` reads current values to derive descriptions.

The demo fixes today/month to 2026-09-14 and mint color #89DDC9. Pure helpers format
month/selection/color, build date presets (demo day/one week later/clear) and color palette
sections (Favorites plus Blue, or all families). These dates are synthetic fixtures, not
current-time queries. State initially displays two inline months, event badges with count 2, no
viewport observations, read-only false, and committed date/color seeds. The
[choice picker](choice_picker_preview.md) is a separate child.

Inline Calendar_controller owns a native Range selection seeded Empty; inline Color_controller
owns its native input seeded mint. Popup controllers instead receive application-owned committed
values/setters. Each opening creates a fresh native draft. Native Apply appointment first
reads/revalidates draft/current opening/value/policy, then calls `set_date`; this updates the
Bonsai model, derives caption/badge/value config and reconciles the native view. Constructing
effects does not apply a date. Selecting a day/preset only edits popup draft; Cancel/Escape
discards it without changing the confirmed value. [Date picker](../../lib/eio/date_picker.mli)
and [color picker](../../lib/eio/color_picker.mli) fence delayed replies against newer openings.

`calendar_appearance` and color appearance derive palette-scaled geometry and paint; color
panels offer Palette/HSLA. `calendar_content` builds passive slot overrides for
arrows/headings/day badges on days 14/17/21, including meaningful descriptions. Viewport
callbacks set optional native observations, which trigger `let%arr` to derive artwork for actual
displayed months/grid dates. They do not change calendar selection or provide a second cursor
owner. Before observation, fixed fallback dates/month are used.

Popup custom triggers are passive badge/caption rows under one native button with explicit
accessible names. Date uses one-month draft, presets and width-420 overlay; color popup
constrains height/scrolling. Clear buttons sequence controller cancel then set application value
Empty, disabled under read-only. Inline month buttons set 1/2/3/12 appearance months without
resetting native selection. Update events assigns a captured next count, deriving slot content
independently of draft. Inline range readout uses controller snapshot, not the committed
appointment value.

Try inline ranges, select a popup draft then Cancel, reopen and Apply, and change event
count/month display while retaining selection. Native GPUIO owns draft/viewport/channel editing
and hover previews, Bonsai owns confirmed popup values/options and adapters own guarded
openings/leases. No appointment service, network fetch or asset registration is started. Adapt
with real current-date input, validated constraints and scoped services, keeping
preview/draft/confirmed values distinct. Read-only is native editing policy, not a promise that
application setters cannot change data.

## Run and review

From the repository root:

```sh
./scripts/gpuio build examples/gallery/main.exe
./_build/default/examples/gallery/main.exe
```

The wrapper uses the repository toolchain. These commands are instructions, not checks run for
this documentation change. The page has no standalone executable or self-test. Compilation does
not establish native keyboard, focus, IME or platform acceptance. See
[gallery instructions](README.md) and [development](../../docs/development.md).
