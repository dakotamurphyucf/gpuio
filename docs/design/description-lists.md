# Rich description lists

`Presentation.Description_list` adds rich structured details alongside the
unchanged `Description.create` / `description_list` convenience API. The reference
is gpui-kit `84f57fdfcb4910623fb0bb7f795b077e249f9271`, captured in the pinned
[DescriptionList source](../catalog/sources/component-description-list.rs.txt).
This source row is a locally validated functional equivalent. The broader catalog
and release acceptance remain open.

```ocaml
let module D = Gpuio.Presentation.Description_list in
let model =
  D.Item.create ~key:(Gpuio.Key.of_string_exn "model")
    ~term:[ Gpuio.View.text "Model" ]
    ~definition:[ Gpuio.View.text "Local" ] ()
  |> Core.Or_error.ok_exn
in
D.create appearance ~columns:1 [ model ]
```

## Validation and layout

Items have stable unique keys, rich term/definition view lists, a span and optional
root/slot styles. Span must be 1..10. The list constructor additionally validates
columns (1..10, default 3), rejects any span exceeding the column count and rejects
duplicate keys, including separator keys. This deliberately rejects invalid input
instead of copying the source's column clamping or unchecked zero/oversized spans.

Packing follows input order. A cell starts a new row when adding its span would
exceed the column count; a separator consumes a whole row. Remaining space in a
partial row is shared equally between its cells beyond their span-based widths.
For example, spans 1 and 2 in a four-column row receive 37.5% and 62.5%, rather than
normalizing the spans to one-third and two-thirds. Consecutive or leading separators
remain intentional full rows. The separator band is 8px tall, plus a following
one-pixel row border when applicable.

The root is a full-width, minimum-width-zero wrapping row with no column gap,
stretched cells and hidden overflow. Cells are direct keyed children; integer
packing determines row borders and first-column styling. Native flex layout owns
measurement, wrapping and distribution. No synchronous measurement callback,
absolute-positioned view tree or per-frame OCaml work is added. Changing columns,
spans, ordering or axis does not add changing row parents around embedded controls.
Caller overrides of placement styles can intentionally alter the default packing.

Horizontal cells place the term beside the definition. Label width defaults to
120 logical pixels, with no shrink; it accepts nonnegative pixels or percentages
and rejects Auto, including when the axis is Vertical. Vertical cells stack the
slots and omit this width. Definitions grow, have minimum width zero and clip
their content. Cells clip horizontal overflow; callers should use suitable label
widths or the vertical axis for narrow layouts.

## Appearance and styles

Borders default on for **both** axes. This follows the source renderer; its
horizontal-only builder comment does not match the implementation. The root uses
a one-pixel Appearance border and an explicit GPUIO 8px radius. Each non-final row
has a one-pixel bottom border. Horizontal terms have a right border and, except
in the first column, a left border. Vertical terms have a bottom border.

Terms use Appearance muted text and, when bordered, the raised surface. Definitions
inherit ordinary foreground on a transparent background. Root typography is 14px
with 125% line height; explicit rich-child styles remain authoritative. These are
GPUIO Appearance mappings, not a dependency on the source's theme registry.

| Size | Bordered slot padding X/Y | Unbordered row gap |
| --- | --- | --- |
| XSmall, Small | 4px / 2px | 2px |
| Medium (default) | 8px / 4px | 4px |
| Large | 12px / 6px | 8px |

Bordered rows have no gap. Unbordered slots have zero default padding and no
default fill or border. Root, item, term and definition styles refine their
respective defaults last; separators have an independent item style. Text-only,
empty and interactive rich slots use the same structure.

## Ownership and accessibility

The root exposes DescriptionList semantics; each entry has stable Term then
Definition parents around its rich children. Separators have Separator semantics.
No extra focus stop or live region is added by the composition. Native controls
keep their own names, actions and focus behavior; applications provide useful
localized labels for embedded inputs and buttons. This structure is not an
automatic form-validation or label-for relationship.

Application state, editors, resources, tasks and I/O stay with the caller. Stable
keys and slot parents preserve native identities across reflow and cosmetic
changes. Removing a slot/item retires its controls; re-adding it creates new native
owners. Ordinary stale-event fencing and theme/style rules continue to apply.

## Current evidence and remaining work

Four Core expect tests pass: range/width/key validation, the source packing example,
ordered semantic slots, leading/consecutive separators, sixteen size/axis/border
cases, style precedence and forty columns/span/axis/order/separator/text updates.
Fourteen embedded controls keep native IDs and current callbacks; equal snapshots
produce no operations and retired callbacks are rejected.

The public **Details that stay together** gallery card exercises these APIs with
twelve equal or seven mixed-span entries, a native editor, term/value actions,
separators, axis, columns, size, four widths, long values, reversal and slot styles.
The repository gallery and fresh installed-library consumer pass 75 packing,
axis, width, size and style cases. Native AXTerm/AXDefinition parents preserve
term/value reading order through reflow and reversal. Four OS Return/Space actions,
the typed draft, native control identity, slot retirement and page remount pass.
The matrix includes 1..10 columns and widths 720/607.3/541/333.3; a speculative
floating-point wrap issue did not reproduce, so no rounding workaround was added.

Thirty-four actual GPU checks cover two themes, both axes, bordered/unbordered
surfaces, all four sizes and custom slot refinement. Padding, label fill, transparent
definition fill and axis-specific term borders match the contract. The final
capture reveals the entire structured list; snapshots supplement geometry/input
assertions rather than serving as the sole acceptance criterion.

Commands on macOS 14.5 arm64, 2026-09-30:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 @all @runtest @fmt
python3 scripts/test_gallery.py --section descriptions
GPUIO_JOBS=2 python3 scripts/test_extension_consumer.py --example gallery --workspace /private/tmp/gpuio-m7-description-consumer-20260930
python3 scripts/test_gallery.py --section descriptions --executable /private/tmp/gpuio-m7-description-consumer-20260930/consumer/_build/default/main.exe
```

Full Dune and both focused native runs passed. Native runs used 480-second
process-group watchdogs, exited zero and reaped their children. The independent
consumer stages installed libraries in its own prefix and builds a locked native
backend without modifying an opam switch. An earlier reading-order harness lookup
used AXDescription instead of AXTitle; correcting the lookup resolved that failure.

The focused section also runs in `core` and `all`; the expanded combined run is
still pending. No hosted/platform checks, VoiceOver, real IME, performance or
clean-machine distribution acceptance is claimed here. Required Linux non-GUI
checks remain; full desktop qualification is OCH-47. This composition adds no
protocol, dependency, native resource owner or GPUI fork change.
