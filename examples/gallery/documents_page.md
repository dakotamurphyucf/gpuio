# Documents page: retained sources and native reader controls

[documents_page.ml](documents_page.ml) and its [interface](documents_page.mli) show
Markdown, HTML, code, diffs and explicit image alternatives. You can append/reset
mock content, change reader presentation, inspect application actions, compare
reader defaults and expand a compact preview. The page uses public document APIs;
only its optional native profile comes from an extension-author Rust package.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
# Optional application-wide reader settings comparison:
./scripts/gpuio exec _build/default/examples/gallery/main.exe --document-defaults
# Optional profile signal log (enable the profile in the page):
./scripts/gpuio exec _build/default/examples/gallery/main.exe --trace-document-profile
# Link/schema diagnostic only; exits without opening a window:
./scripts/gpuio exec _build/default/examples/gallery/main.exe --check-catalogs
```

Choose **Markdown & code**. Begin in Markdown, append a finding, activate a link and
expand the bottom preview. Then compare HTML/Diff/Image alternatives. There are
no file/network prerequisites: sources and PNM image bytes are fixtures. The
[development guide](../../docs/development.md) explains the isolated toolchain;
[profile package](../document_profile_package/README.md) explains required compiled
registration. Catalog checks establish linkage/schema compatibility, not real
native controls. `--background` avoids requesting focus for layout work. Current
release scope is macOS-first; this review does not establish new keyboard,
clipboard, Linux GUI or VoiceOver qualification, including the optional profile.

Read `Resources`, `Mode`/`Defaults_policy`, then `component`. Follow its local
`run`, config construction and `attach_profile` before reading the view callbacks.
Supporting [Preview_scope](preview_scope.md) owns visit acquisition/disposal;
[Diff_state.ml](model/diff_state.ml) and its [interface](model/diff_state.mli) own
application diff policy. [application.ml](application.ml) installs immutable
application defaults; this page only chooses per-view overrides.

## Source owners, revisions and bounded mock updates

`Resources.t` contains six `Gpuio_eio.Document.t` registrations (Markdown, profile
preview, HTML, code, diff and image examples), one borrowed asset handle and four
fragment counters. `Resources.create` chains asynchronous effects, mapping typed
errors and stopping on the first failure. All acquired registrations share one
`Preview_scope` child, so failure or departure retires partial acquisitions too.
`Text_source.of_string` gives Markdown/HTML/code/diff Streaming status initially; the
others are ordinary complete fixtures. Streaming here describes source state, not
a network producer: this page never starts a fetch loop or calls `finish`/`cancel`.

`append` adds at most six Markdown findings; `append_html` adds six HTML paragraphs;
`append_code` adds six OCaml bindings; `append_diff` adds three Rust-file patches. They update counters only after the
local `D.append` succeeds and become idempotent at the limit. The reset helpers
restore the corresponding fixture and counter. These are UI-domain operations:
[document.mli](../../lib/eio/document.mli) distinguishes coalesced desired snapshots
from native acceptance, parser completion and physical presentation. `D.handle`
passed into a view is borrowed; it cannot outlive the scope. The page owns source
text, while native readers own parsing, retained layout, selection and scroll.

The strings are intentional demonstrations: Unicode, simple/unsupported YAML,
MDX child text without evaluation, registered/missing images, links and multiple
files in a diff. `asset://prism` resolves only through the explicit configuration
mapping to the registered PNM handle. The external HTML image URL and missing
Markdown asset remain alternatives; there is no implicit HTTP image fetch. The
intro's `asset://gallery-link` is also unmapped. User-supplied source should be
validated through the public constructors rather than copying fixture `ok_exn`
wrappers into a parser boundary.

## Bonsai graph and derived reader configuration

`component` receives the app/window, reactive palette and Bonsai graph.
`B.state` gives each typed choice or notice a reactive value plus a setter effect;
`B.toggle` gives each Boolean a toggle effect. Initial mode is Markdown;
Defaults_policy is Inherit. Most presentation switches are false, descriptions,
enable-actions and native-copy are true, and the compact preview starts capped
at six lines. `B.state_machine0` gives diff actions to `Diff_state.apply`, starting
with native-managed expansion, word diff enabled and a four-line seed.

`let%arr` combines current resources, palette and choices to derive GPUIO views.
An effect is scheduled work, not something run while building a view. For example,
`run` wraps a source operation in `E.of_thunk`, then `E.bind` sends its result to
`set_notice`. The lifecycle effect resets notice and diff state on departure;
other choices remain retained. Scope cancellation separately destroys the documents;
returning preserves controls while reacquiring fresh sources and counters.

The mode match selects a source and `Document.Mode`: Images is Markdown with image
bindings, Code selects the OCaml language and path `greeting.ml`, and Diff supplies
`Diff_state.config`. `Document.Config.create` supplies appearance, actions, selection
format, search text and a native viewport height (350 logical pixels, 450 for Images).
The main `V.document` key is `Mode.label mode`, preserving identity for updates in
that mode and distinguishing readers across mode changes.

`refined` constructs `Document.Style` with colors, six heading sizes, 1.4-rem
paragraph gaps and code/table refinements. Markdown options expose frontmatter as
disabled, descriptions or code, and MDX as syntax without execution. The selection
format switch controls native text selection copy, while Copy source/code/table
retain their separate semantics. Search highlights `let` without editing source.
Relevant contracts are [document.mli](../../lib/core/document.mli),
[text_source.mli](../../lib/core/text_source.mli) and the
[reader styling design](../../docs/design/document-styling.md).

## Concrete updates, navigation and actions

Click **Append a finding** in Markdown. Its native button schedules `run`; that
calls `Resources.append`, creates Finding 1 and requests `D.append`. The local
counter becomes one, and its returned message updates notice through a Bonsai
effect. Native publication/parsing/layout later incorporates the new source revision.
The main Markdown reader and bottom preview share that same source handle, so both
can reflect the append; no duplicate document copy is made. Six clicks reach the
fixture limit; later clicks return the same count without appending. **Reset document**
requests the original intro again and clears the counter.

In Code, **Append code** calls `Resources.append_code` on `resources.code`,
adding `let finding_1 = "Useful detail 1 · 世界"` and advancing its separate counter.
Six additions reach the cap. **Reset code** calls `reset_code`, restoring the
original greeting fixture and that counter. These operations leave Markdown and
its bottom preview unchanged; Markdown's controls likewise leave code unchanged.
Code begins with Streaming status so `D.append` is valid even before its first
reset. Native code readers stay read-only: source publication is an application
operation, not an editable text-input API.
**Try unsupported YAML** replaces the Markdown source with quoted/sequence syntax;
it does not reset the fragment counter. Reset document restores both original text
and count. Enable frontmatter to compare the unsupported source fallback.

Native link activation invokes `on_navigate`, returning URL plus input metadata;
line navigation returns path/side/line. The handler records a notice and opens no
browser or file. Markdown/HTML `Document.Actions.Config` optionally adds typed
`inspect` and `summary` IDs for code/table blocks, with enabled state independent
of visibility. `on_action` displays the native snapshot's byte length/language or
column/row count, source revision and activation. Native Copy buttons copy locally
without dispatching those custom actions. See
[document actions](../../docs/design/document-actions.md) and
[link activation](../../docs/design/document-link-activation.md).

In Diff, native observations inject `Diff_state.Action.Observe`. Managed mode keeps
configuration seeds stable while native code changes collapse/line limits. Toggling
**Application controls expansion** makes the model authoritative; a Show more
request updates its limit in four-line steps (bounded at 8192), and `let%arr` submits
the resulting controlled config. Reset diff also injects Reset. Stable file keys,
not row indices, identify collapsed files; the model avoids replaying already-applied
observations or incompatible ownership combinations. See [diff_state.ml](model/diff_state.ml) for those guards.

## Profiles, defaults and compact previews

`attach_profile` wraps Markdown/HTML views when **Native document profile** is on.
`Profile.instance ~generation:1L` chooses Indigo or Amber properties; this typed
instance belongs to the [document-profile package](../document_profile_package/README.md).
Its callback receives revisioned typed signals and updates the notice. The optional
`--trace-document-profile` prints those signals as `GALLERY_DOCUMENT_PROFILE`;
it does not enable the profile by itself. A second Flow-layout profile preview
contains review badge/card syntax and its own native controls. Pure OCaml readers
need no Rust knowledge; implementing that extension package does.

The Markdown-only defaults card renders that same profile-preview source with
`V.without_document_profile` to isolate reader settings. Inherit leaves app defaults
intact. Builtin uses `Document.Config.with_overrides` to reset text style and
selection format explicitly. Compact overrides paragraph gaps to 0.4 rem and copy
to Plain_text. `--document-defaults`, handled by startup, instead sets inherited
paragraph gaps to 1.8 rem and Markdown selection copy. These per-field policies are
in the [defaults contract](../../docs/design/document-defaults.md); changing them
does not mutate application defaults or source text.

The bottom reader has key `line-preview` and Flow layout. Its expand toggle changes
`max_lines` from Some 6 to None. `on_preview` reports Pending, Collapsed,
Source_view or Rich with the clamped flag; the notice describes a native observation,
not an OCaml line count. See [preview layout](../../docs/design/document-preview.md).
No per-frame Bonsai timer or polling task is needed for layout, copy or scrolling.

For a small adaptation, add a bounded paragraph to `Resources.append` or change the
validated reader style. To register another document image, extend the scoped
resource record and explicit URL-to-handle mapping; do not rely on HTML URLs to
fetch it. Preserve source handles, stable keys and revisioned event semantics,
and keep async acquisition/cancellation in `Preview_scope` rather than tying source
ownership to a virtual native row. To connect a real feed, acquire explicit Eio
capabilities and use a scoped producer with deliberate finish/cancel behavior.

Read the [diff-state walkthrough](model/diff_state.md) for the pure reducer
and managed/controlled observation guards, and the [image fixture walkthrough](image_samples.md)
for the encoded raster shared by these document previews.
