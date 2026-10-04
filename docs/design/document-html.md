# HTML document reader — OCH-41

`Document.Mode.Html` uses the same registered revisioned source and native
document presentation as Markdown. It accepts basic reader markup: headings,
paragraphs, emphasis, marks, links, images, lists, quotes, rules, tables and
preformatted code. Finite nonnegative image/table dimensions and the pinned
mark-color subset are supported; this does not enable general CSS layout.
It is not a browser: no scripting, stylesheets, forms or implicit resource
acquisition. Embedded HTML in Markdown retains its literal-text behavior.

Parsing runs in the existing bounded latest-snapshot worker pool. Rich input is
limited to 64 KiB, 16 KiB per line, 4096 DOM nodes, depth 32 and 256 top-level
display blocks. Exceeding a limit uses the existing bounded source view. The
source is preserved verbatim for explicit Copy source. Selected content copies
plain text even when Markdown selection format is requested: HTML normalization and
entity decoding do not provide original byte offsets for partial selections.

Every parsed image is lowered to GPUIO's registered-asset renderer before the
prepared document can reach layout. URLs are exact keys in `Config.images`;
unmapped images use their alternative text. Links use queued navigation events,
including links enclosing images. Unknown attributes and browser behaviors are
not interpreted. Mode changes are part of worker identity; stale preparation
cannot replace the current picture. Source replacement conservatively clears
HTML selections because HTML has no reliable original source spans.

Flow/Viewport, search, collapse and explicit source presentation use the existing
document APIs. Preview line budgets and observations support HTML Flow with the
same identity, clipping and event rules as Markdown. This document specifies the
implementation contract; test and physical acceptance are recorded separately.
