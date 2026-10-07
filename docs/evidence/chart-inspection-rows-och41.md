# Structured inspection rows from ordinary Views

OCH-41, 2026-10-06. Local stock OCaml 5.3/Core/Bonsai v0.17 work after `7de57dc`.
The public interface was drafted before implementation. This adds an OCaml
composition helper, with no new native widget, wire schema, dependency or callback
path. It does not qualify its final gallery appearance or complete the catalog.

`Presentation.Chart_inspection.Row.create` accepts a stable key, swatch color and
ordinary rich label/value Views. `Row.text` builds the common text case. The `view`
helper takes an explicit Presentation.Appearance, optional rich title and root/row
style refinements. It composes existing containers/text and preserves child
callbacks, ordinary View limits and the application's ownership of state/resources.

Duplicate row keys fail before mounting. Title and row children have separate key
namespaces, and label/value content has stable internal wrappers. Label/value
wrappers expose Term/Definition semantics; the decorative empty swatch has no
label, callback or focus stop. The enclosing native Card supplies its backing;
Overlay keeps its plot-sized container. Rich child Views can override inherited
colors/fonts or contain ordinary controls. The helper owns no selection, native
source, tasks or editor controllers.

Meaningful expect tests cover duplicate rejection, empty rows and optional
localized/rich titles in both built-in appearances. A reconciliation test mounts
three rich button values inside an actual public `View.chart` description using
an explicitly owned test resource handle. Twelve updates reorder rows, toggle
its title, alternate appearances and refine styles. They preserve all button IDs,
refresh callbacks, emit no chart metadata update and produce no operations on an
identical subsequent render. Unmount rejects the old callbacks. Row keys named
`title`, `rows` and `label` deliberately exercise separation from internal names.
This is public API/reconciliation evidence, not actual GPU or OS input evidence;
the resource handle is an injected fixture rather than a native registration.

Commands through the repository-isolated environment:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune runtest test/view_api -j 2
GPUIO_JOBS=2 ./scripts/gpuio exec dune build @all @runtest @fmt -j 2
```

Both commands passed locally on macOS 14.5 arm64 (Apple M1 Max). The full Dune
check includes build, expect tests and formatting; no foreground window was
needed for these checks. The [raw logs](chart-inspection-rows-och41-logs.tar.gz)
and [SHA-256 manifest](chart-inspection-rows-och41-manifest.json) retain five
members (706,492 uncompressed bytes), including exact commands and hashes of
the three tested implementation/interface/test files against base `7de57dc`.

The rendered gallery example, light/dark visual inspection, native controls and
root/fresh-installed consumer walkthroughs remain required. Earlier native
inspection tests qualify the arbitrary-child adapter's specific behavior, not
this helper's final layout. Hosted CI37549499328 covers `54173d2`, before this
helper and several local native-test checkpoints. OCH-41/OCH-17 remain open.
