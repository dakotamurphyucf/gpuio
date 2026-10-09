# Static document profiles — OCH-41

Implementation contract; public attachment and native worker/renderer integration
have local evidence, including a public gallery/package and installed consumer.
Application defaults also have runtime/consumer evidence; virtual controls have
production-host focus tests, with physical and broader stress acceptance open. This extends the existing document reader and does not replace
the general component SDK.

A statically linked **profile** groups a qualified schema, bounded typed properties,
optional fenced-code highlighter, code/table action renderers and named Markdown
block/inline plugins. The same schema/version/fingerprint must be declared by its
OCaml package and Rust factory. Ordinary applications use a typed package wrapper;
raw bytes, factories and parser objects stay behind that wrapper. A profile applies
to one DocumentView. Multiple plugin families compose inside a profile, avoiding
ambiguous renderer precedence between independently attached profiles.

## Public shape and ownership

The checked OCaml interface is [Document.Profile](../../lib/core/document_profile.mli).
It has Core/bridge/native attachment tests and an independently installed gallery.
Reuse the existing validated Extension.Schema and bounded bin_prot Codec. A profile
definition has properties and events, with no imperative command queue: property
updates and source updates drive preparation, and native callbacks enqueue events.
Attach an instance through `View.with_document_profile view instance ~on_event`.
This modifier keeps the event codec coupled to its instance without putting an
existential decoder into ordinary Document.Config values. Only rich Markdown/HTML
DocumentViews admit profiles; Markdown AST plugins do not turn HTML into Markdown.

Each retained View has a monotonically increasing host configuration epoch. An
application instance generation explicitly requests reset; property changes also
retire prior callbacks. Clear/reinstall does not reuse epochs. Source identity,
source generation/revision, profile epoch and prepared interpretation participate
in worker/result and event acceptance. Render closures may refer to an old picture
while an append prepares, but cannot describe it as the new revision. Eio checks
the source registry before callback dispatch. No OCaml callback runs inside Rust
parsing, layout, paint, native delegates or worker tasks.

The `gpuio-document-sdk` crate depends on the pinned Base text API and GPUI. It is
separate from the lightweight component SDK, while reusing its revocable event
leases. Registry construction snapshots and validates descriptors atomically.
Property validation is pure and bounded; profile configuration and all parsing/
highlighting run in the existing two-worker document pool. Hooks receive explicit
cancellation, never a Window/App on the worker. The completed profile and prepared
text install together. Rendering hooks receive native Window/App and a revocable
event context stamped with installed source provenance. Native resources remain
Rust-owned and their lifetimes must follow prepared/view owners.

## Text, rendering and failure contracts

Plugins explicitly declare Text, NonText or Opaque presentation. Text is rendered
by the reader's own glyph/selection/search machinery; it does not permit replacing
those glyphs with an unrelated arbitrary element. NonText promises no searchable
glyphs. Opaque permits custom native content but makes complete document search
counts unavailable. Native renderers must supply truthful semantics and clipping,
and cannot inherit searchable/selectable status from a source label alone.
Structured custom text projection needs separate implementation/evidence, rather
than weakening this invariant. Plugin source spans are provenance, not persistent
editable AST IDs. Native action renderers receive full CodeBlock/TableData.
The [mounted semantics checks](../evidence/document-profile-semantics-och41.md)
cover actual search counts/paint, select-all copy and native AX roles across
presentation changes. A copy label is independent of displayed-text eligibility;
it does not make a NonText or Opaque object searchable.

Highlighters return sorted, nonoverlapping, nonempty UTF-8 byte ranges with concrete
colors, font weight, italic, underline and strike flags. Validate the whole result
before installing any colors; no partially accepted prefix. Bound input, language,
run count and aggregate work across code blocks. Check cancellation before and
after every hook. Panics become typed failures at host hook boundaries. Arbitrary
unsafe Rust, aborting panics, memory allocation inside trusted plugins and hooks
that ignore cancellation cannot be sandboxed or preempted by this API. Host byte
reservations and plugin-declared retained allocations must be accounted together;
a bounded wire payload alone does not prove bounded native resources.

A parser/plugin/highlighter failure preserves the original source and exposes a
typed failure/fallback; it must not silently drop content. A render/input failure
revokes the affected instance and reports it once. Plugins cannot execute code or
fetch URLs by virtue of receiving Markdown; ambient I/O remains outside the reader
contract. Static Rust authors remain responsible for their explicitly added code.

## Composition and remaining implementation

Host image/literal-HTML safety adapters stay installed. Plugin names are qualified
and cannot collide with each other or internal names. Parser order and whether a
plugin may intercept built-in syntax must be explicit. Renderer-only refreshes
retain parser identity, prepared text and selections; parser/property changes
reprepare. Logical keyboard order includes offscreen virtual controls through
bounded realization and guarded focus transfer. Read-only measured bounds are
reveal hints, while a later painted frame supplies actual focus eligibility. The
host owns composite/modal exit policy. See the [virtual focus checkpoint](../evidence/document-virtual-focus-och41.md)
for tested cases and remaining qualification.

### Plugin-owned scroll areas

The reader reveals blocks in its own virtual list. A plugin that embeds an
independent scroll area owns that area's offset and keyboard reveal policy.
Measured descendant bounds do not authorize the reader to change an arbitrary
inner scroll container. Only descendants eligible in a painted frame can receive
focus; a clipped candidate gets a bounded outer reveal attempt before traversal
continues. This input clipping also applies to `Layout.Flow` documents:
plugin-owned viewports must exclude hidden controls even when the enclosing
reader has neither its own scrollbar nor a preview line limit. See the
[flow-layout regression and macOS checks](../evidence/document-profile-flow-focus-och17.md).
Leaving the reader follows the host's composite order. Reverse entry
from the toolbar returns to the reader anchor, rather than implicitly selecting
the last plugin control.

A plain scrolling `div` is not a complete keyboard-accessible composite. Plugin
authors must provide keyboard scrolling/reveal or equivalent accessible controls
for content that would otherwise require the pointer. Keep focus and scroll
state in native keyed/entity state, preserve the renderer's source/event lease,
and allow traversal to leave the composite. The [nested-scroll test](../evidence/document-profile-scroll-och41.md)
qualifies clipping, exit, wheel reveal and queued activation for an independent
viewport; it does not establish automatic inner reveal or physical accessibility
for arbitrary plugin widgets. The public [review profile](../../examples/document_profile_package/README.md)
now supplies a bounded `review-scroll` example with Show start/end buttons outside
its inner viewport. These native keyboard-accessible alternatives demonstrate the
plugin-owned policy; they do not change the reader into a controller for arbitrary
plugin scroll state. Its handle lasts only while the occurrence remains rendered.

Backend manifests register document profiles separately from component factories
through optional `document_profiles` entries (`path`/`factory`). `components: []`
is valid when profiles are present. A package supplying both categories uses one
Cargo dependency identity. Generated backends validate and install both catalogs
atomically through `gpuio_native::registrations::install`; invalid registration
leaves both catalogs open for correction. Either legacy category-specific installer
or the first catalog query freezes both catalogs, so combined backends must call
the combined installer before creating the transport or querying catalogs. Verify
a standalone installed consumer with parser, inline renderer, code/table renderer,
highlighter, queued event and shutdown examples.

[Application defaults](document-defaults.md) implement Base fallback < application
settings < explicit per-document overrides. Constructors now preserve omission,
and per-field Inherit/Builtin/Value intent survives resolution. Immutable defaults
flow through the application, window driver and reconciler; typed shared callbacks
receive the resolved source Config. No process-global mutable singleton is used.

Acceptance still requires protocol/Core/Bonsai/Eio attachment, generation/failure
routing, pool/cancellation/resource accounting, parser/renderer/search/selection/
copy/accessibility/focus behavior, gallery, independent consumer, reconstruction
and macOS qualification. A standalone SDK build establishes none of those host
claims by itself.

## Bridge implementation decisions

Op121 carries a positive host epoch and an optional instance (schema, positive
application generation, bounded property bytes). Clearing retains the host epoch
watermark. Instance generation may not decrease while the same schema remains
installed; clear/reinstall is a new installation with a strictly newer host epoch.
All profile instances require a document event handler. Event78 carries that epoch,
instance generation, positive published source generation/revision and either
bounded data or a typed stage/error. Unpublished revision zero does not run profile
preparation: it waits for source publication, so no unrouteable zero-revision
configuration failure is emitted. Catalog/schema admission failures reject the
transaction; the separate catalog query enables package preflight diagnostics.

Descriptor `max_retained_bytes` declares additional opaque native retention per
installation, at most 4 MiB. Host reservations must add this to bounded source,
generated text, highlight and adapter metadata allowances before configuration.
A trusted author's declaration cannot measure arbitrary Rust allocations, so
resource qualification must verify actual implementations too.

## Worker attachment decisions

A worker request owns its checked profile binding and complete wire configuration.
Both participate in request identity; updates cancel prior work before any replacement
can publish. An unpublished profile source waits without scheduling a worker.
A completed result reduces its work reservation to a checked retained-picture
allowance. Published metadata counts generated public strings, displayed text,
plugin metadata, code keys and actual highlight vector capacities plus declared
opaque state. If retained capacity exceeds the admitted reservation, reject the
whole prepared result before publication. Retained render closures must share the
charge with their picture; releasing the Presentation alone must not release a
still-live closure's reservation.
Reservation includes ordinary source/AST work, declared opaque retention, bounded
generated strings, display projection and complete custom highlight storage. These
are conservative admission units, not measured allocator RSS.

SDK preparation returns a typed Parse/Highlight stage alongside its error so a
highlighter panic cannot be mislabeled as a parser failure. Configure is a separate
worker stage. Native fallback highlighter runs and SDK custom runs remain distinct;
the custom font weight and strike flags are preserved through native conversion.
A per-handle worker serial supplies the positive parser epoch. UI-only image/render
closure refreshes retain that prepared epoch.


## Installed renderer ownership

The native Presentation binds node metadata before submitting worker requests.
Worker identity includes the event handler as well as the profile configuration;
changing a handler alone must replace the installed callbacks. A completed picture
installs prepared text, full highlight styles, plugin adapters and native action
renderers together. Image-only closure refreshes reuse the same installed profile
and parser epoch, preserving text selection.

An installed profile owns an EventLease and an immutable source/configuration/
handler stamp. Transport references are weak and callbacks enqueue Event78 without
entering OCaml or borrowing the UI tree. Property changes, handler replacement,
source generation reset, clearing, unmount and eviction close the old lease even
when cached native elements still hold the old picture. An append can keep that
picture interactive while preparation runs; its events carry the old installed
revision, never the pending revision. Eio applies its existing source fences.

Root paint first hides all installed leases, and each visible rich document enables
its lease within its content mask. Collapsed/source-only/inert views remain hidden.
Pointer policy is independent: renderer authors use `guard_pointer` for mouse/touch
and `guard` for keyboard/accessibility, retaining keyboard operation when pointer
input is disabled. Native child elements still own clipping, focus and semantics;
a document-wide lease does not substitute for correct child hit testing.

Render failures report once and close the instance. Guarded input panics close the
lease immediately and report on the next host render/lifecycle boundary. UI plugin
adapters report failures through the same weak owner and display an alert fallback.
These boundaries contain unwinding panics, not aborts or arbitrary unsafe code.

The shared retained-picture charge travels with the installed profile, adapted
render contexts/event sinks and cached code highlighter. Retained sinks hold that
charge independently of their weak revocable event owner. Closing/unmounting
prevents delivery immediately; accounting is released only after the last resource
holder drops. No strong transport or Presentation reference is introduced by a
sink. Resource reservations are conservative host accounting, not measured RSS.


## Public authoring example

The [native review package](../../examples/document_profile_package/README.md)
keeps typed Accent/Event modules and validated codecs on the OCaml side, and uses
only `gpuio-document-sdk` on the Rust side. The SDK re-exports the exact pinned
Base primitives as `sdk::base` for native elements; this is source-level dependency
compatibility, not a stable binary plugin ABI. The gallery's generated backend
registers both a component and a document profile atomically. Its no-window
`--check-catalogs` mode verifies both schemas in a fresh linked process.
[Independent-consumer evidence](../evidence/document-profile-package-och41.md)
distinguishes installed build/codec/catalog checks from physical GUI acceptance.

## Declared block text selection

A block plugin declaring `Text` uses the reader's normal glyph selection, with
selection state owned by that parsed occurrence and retained across redraws.
Source replacement and explicit selection clearing retire that state. Selecting
an entire projected block in Markdown copy mode uses its declared Markdown;
a partial glyph range copies the selected display text because arbitrary plugin
output has no character-by-character mapping to source syntax. Plain copy uses
the selected display text. Whole-document copy retains the existing declared
copy-text/source contract. Inline custom objects retain their atomic selection
contract; this does not introduce selection inside an arbitrary custom renderer.


The logical selection projection now addresses declared block glyphs even when
`MarkdownNode.text` specifies a different whole-document Copy representation.
It does not expose that alternative as invisible selectable glyphs. Whole Copy
and partial glyph Copy retain the distinction above through compatible streaming;
empty presentations still retain independently bounded declared Copy data. See
[the selection projection contract](rendered-document-selection.md#declared-custom-block-glyphs-versus-whole-document-copy).


Opaque/NonText block renderers now have whole-object selection around their
native layout, including empty Copy alternatives. Child controls retain their
own interaction; custom extension controls must claim pointer selection through
the native suppression/editor adapter described in the
[block ownership contract](rendered-document-selection.md#opaque-and-nontext-block-ownership).
No character mapping or rich AX selection is implied for arbitrary native content.
