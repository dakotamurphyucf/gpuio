# Markdown parser options — OCH-41

`Document.Markdown_options` is immutable parser configuration, supplied through
`Document.Config.create ?markdown_options`. Defaults retain the existing GFM
reader. Nondefault options require Markdown mode; clearing options and changing
mode is one atomic transaction. HTML reader configuration remains separate.

`Frontmatter.Disabled` leaves frontmatter detection off. `Code_block` enables
leading YAML frontmatter and preserves its contents as a syntax-highlighted YAML
code block. `Description_list` renders the restricted top-level metadata mapping
as native label/value rows, falling back to code for unsupported input or more
than 128 entries. See [the renderer contract](document-frontmatter.md); neither
mode is a general YAML processor.

`mdx=true` enables the pinned parser's JSX and expression AST constructs while
disabling raw HTML. No JavaScript runs, imports are not executed, and no component
is dynamically instantiated. Unclaimed JSX contributes its parsed children;
inline expressions display their expression text, while flow expressions render
as code. Source copy retains original delimiters. The GPUI Base adaptation preserves
inline JSX children and lowers flow JSX children as blocks; the pinned conversion
otherwise dropped inline children and flattened or lost block structure. Unsupported/malformed input
uses existing bounded source fallback. Static parser/renderer plugins remain a
separate required integration contract.

Op119 appends `Set_document_markdown_options(node, {frontmatter; mdx})`.
Frontmatter tags0/1/2 mean Disabled/Code_block/Description_list; mdx is a strict
Boolean. Previous
operation and source publication bytes are unchanged; paired runtimes rebuild.
The fixed-size value lives with the generation-checked DocumentView and resets
on reuse. Unchanged options are silent; removing an override emits defaults.

Worker equality includes options alongside source snapshot/mode/appearance/search.
Changing options cancels or supersedes old results through the existing request
serial, without changing the canonical source revision or native view identity.
The old installed picture remains geometrically stable during preparation, with
its matching renderer configuration. New options install atomically with prepared
content; selection and keyboard-link targets from the old interpretation clear. A fresh
installed-interpretation identity fences old focus/click closures even when source
revision is unchanged or settings cycle back to earlier values.
The outer document, toolbar, source lease and scroll owner survive. Copy source
always uses original source. Image-only renderer updates use the installed parser
configuration and must not trigger an unbounded main-thread parse.
