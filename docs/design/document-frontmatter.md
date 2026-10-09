# Frontmatter descriptions — OCH-41

`Document.Markdown_options.Frontmatter.Description_list` (Op119 frontmatter tag2)
adds the pinned component's read-only restricted YAML metadata view. Disabled
and Code_block tags0/1 retain their meaning. This is parser configuration, so the
existing worker identity, stale-result rejection and atomic installation contract
applies; switching it does not republish source or replace the native view.

Accepted input is a leading top-level mapping with ASCII identifier keys, plain
values, empty values, and literal `|-` / folded `>-` block scalars. Preserve the
pinned parser's whitespace/comment rules. Quotes, compounds, nested mappings,
aliases/tags, inline comments and unsupported scalar syntax fall back to the
original YAML code block. At most 128 entries render as descriptions; larger
mappings use code fallback. Existing source, line, AST and displayed-text limits
still apply. No YAML evaluation, I/O or ambient resource lookup occurs.

A structured prepared native block holds one label and value Paragraph per row.
Labels include their visible colon. Rows use a bounded two-column layout, a label
column up to 12 rem and 40% of available width, wrapping values, and inherited
reader colors/typography. They retain the document's selection and scroll owners;
there are no independent editor widgets or synchronous OCaml callbacks.

Every painted label/value has matching prepared text fragments. Displayed-text
search treats them as separate groups; literal source search keeps its existing
source semantics. Plain copy joins selected labels and values in row order.
Partial Markdown copy reconstructs readable selected text without fabricating a
valid partial YAML mapping; select-all and explicit source copy retain exact
original source through the existing source-copy path. Compatible body appends
retain metadata selection; changed/reordered metadata must not attach selection
to different values. DescriptionList/Term/Definition semantics contain the actual
text, and native clipping/preview rules still govern visibility and actions.

Unsupported syntax stays visible as code, with original source available. The
renderer is a built-in structured document block; it does not establish arbitrary
static plugin/rendering support, which remains a separate required catalog item.
