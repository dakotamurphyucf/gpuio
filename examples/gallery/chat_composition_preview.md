# Compose message slots without owning a conversation service

[chat_composition_preview.ml](chat_composition_preview.ml) and its
[interface](chat_composition_preview.mli) demonstrate stateless Presentation.Bubble/
Message helpers around a native document and editor. Read Inset/Resources,
resource acquisition/editor, presentation states, final `let%arr` and slot construction.
`B = Bonsai.Cont` constructs reactive state, `E = Bonsai.Effect` defers actions,
`V` describes GPUIO views, `D` owns a scoped document and Editor owns one native
text-editor placement. There is no network, provider or message-send service.

After [setup](../../docs/development.md), from the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section chat-composition
```

Find the composed message in Presentation. Toggle typed bubble/reaction, Ghost,
insets, alignment and Markdown; type into Message draft and append fixture chunks.
The physical macOS harness is separate from compile/source coverage;
[README](README.md) records platform limits. This review does not execute it.

## Scope-owned document and caller-owned editor state

`Resources.t` holds `D.t` and mutable chunk count. create publishes a `Text_source`
marked `Streaming` with Unicode and an OCaml code fence; it converts typed document
failure to Core.Error. Preview_scope.acquire owns a fresh child of this window
scope while the preview branch is active. Leaving Presentation cancels/releases
it and clears notice; returning acquires a new initial document/count.
`Resources.append` accepts at most six deterministic complete-string updates, using
`D.append` and incrementing count only on local success. The cap is not finish:
the source stays `Streaming`. `D.append` coalesces desired updates; acceptance is
not parser completion or paint, per [Document](../../lib/eio/document.mli).

`Editor.create` seeds Keep this draft in a `Single_line` native editor. This is outside
Bubble/Message slot construction. Stable draft/payload/body-action keys preserve
placement when neighboring optional slots or styles change; native editing/IME/
selection/undo belong to the [editor](../../lib/eio/text_input.mli), not the Bubble
helper. Page departure unmounts that lease; this preview does not mirror edited
draft text to a persistent application model for later reseeding.

## Reactive controls feed pure presentation helpers

`state_machine0` cycles seven variants starting Secondary, optional bubble alignment
Inherit → Start → End and inset Inherit → Yes → No. `B.toggle` supplies message alignment,
reaction side/alignment, avatar/header/footer visibility, footer expansion, typed
versus arbitrary wrappers, Markdown, compact width and clipping. Action/reaction
state machines count explicit clicks. `let%arr` combines palette/resources/editor
and those changing values to produce a view; it does not execute appends/clicks.
Loading/Failed resource states show messages instead of binding absent handles.

[Presentation contracts](../../lib/core/presentation.mli) explain the typed distinction:
`Bubble.create` retains variant metadata. `Message.Content.Item.bubble` carries that
metadata, allowing Ghost to remove inherited header/footer content inset; wrapping
`Bubble.view` as Item.element supplies an arbitrary view and loses that inference.
Explicit content_inset=true/false overrides either case. `Bubble.with_accessibility`
adds Group semantics without losing variant metadata. Message alignment controls
the avatar/body row; an explicit bubble alignment controls that bubble independently.

Reaction Item.action produces a typed ordinary button with pill radius and removes
default decorative wrapper padding. Item.element preserves the supplied ordinary
button style instead. Reaction side/alignment changes retain surviving controls.
Reactions are absolute and do not enlarge measured row height; the preview reserves
24-pixel top margin/padding and optionally demonstrates clipping at ancestors.
The reaction action increments only reaction count; body action increments its own.
No implicit message role, live region or focus stop is added by `Message.create`;
this preview supplies Group accessibility labels for slots/content itself.

## Body modes and a concrete interaction

Plain mode derives text from chunk count; Markdown mode mounts View.document on
the same scoped source using Flow layout and palette appearance. The plain text
is an illustrative parallel summary, not the exact Markdown update bytes. Toggling
mode replaces the body view while keeping the separate draft/action siblings keyed.
Message slots include optional 32×32 avatar, header, typed content and footer;
footer expansion occurs outside avatar/body row. Compact changes width from 560 to 360,
with `Max_width` set to 100%; style changes do not re-create the document registration.

Trace: Append message chunk effect calls `Resources.append` → desired source updates
and count increments → `set_notice` updates reactive state → `let%arr` rebuilds plain
text or same-handle document configuration → native layout grows. Typed message/
reaction actions remain normal deferred callbacks. There is no Eio producer loop
or automatic append timer, and six appends do not certify six native frames.

To adapt, preserve separate document/editor ownership and stable slot keys. For an
actual conversation, introduce validated message data and scoped producers outside
presentation helpers; complete/cancel source status explicitly and mirror drafts
if they should survive native destruction. This finite fixture demonstrates
composition/layout and contains no independent pure component test or new native
acceptance evidence.
