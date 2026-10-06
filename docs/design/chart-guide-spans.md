# Partial chart inspection guides

OCH-41, 2026-10-06. Implemented with [scoped local qualification](../evidence/chart-guide-spans-och41.md).
Broader catalog and release acceptance remain open.

`Chart_inspection.Span` is an immutable validated presentation value. Full keeps
the existing complete plot extent. Pixels supplies a logical-pixel start/length;
Fraction supplies a start/length relative to the current plot height or width.
Vertical guides run top to bottom and horizontal guides left to right. Neither
changes names or direction when the chart's value orientation is reversed.

`Crosshair.create ~vertical_span ~horizontal_span` configures both independently.
The vertical extent uses plot height; horizontal uses plot width. Defaults are
Full. Start may be outside the plot; native layout intersects `[start,start+length]`
with the relevant physical extent. Empty/zero intersections draw no guide. This
clipping rule also applies to thick solid bands and dashed strokes. A span does
not move the guide perpendicular to its axis: that coordinate remains the
inspected mark anchor. Card placement, markers and original selection are separate.

Pixel starts are finite in ±32768 and lengths in [0,65536]. Fraction starts are
finite in [-1,1] and lengths in [0,2]. Lengths cannot be negative. These bounded
values allow partial and overhanging intervals while avoiding unbounded geometry.
Fractions adapt to native resize without an OCaml layout callback or source update.

The wire uses Full=0, Pixels=1 followed by start/length doubles, Fraction=2 with
start/length doubles. Crosshair appends vertical then horizontal spans after its
color field. Parent chart style advances to -7. Matching packages are required;
old schemas are rejected. Maximal styles must still fit the existing envelope.

The pinned `CrossLine.span`/`h_span` API supplies the corresponding physical
functionality. Fractions are an additional declarative convenience. This does not
expose arbitrary tooltip rows or per-datum annotations; those remain catalog work.
Qualification must include independent clipping calculations, malformed/paired
bytes, actual guide pixels, public controls and resource cleanup.
