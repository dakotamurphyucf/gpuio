# Embedded command palettes

Implemented locally under OCH-41. [Native/API and installed macOS evidence](../evidence/palette-embedded-och41.md)
records qualification and its limits; consolidated family/release acceptance remains open.

`Command_palette.Config.create` and `create_entries` accept
`~presentation:Embedded`; `Modal` remains the default. The existing
`View.command_palette`, rich slots, registry, observations and Eio controller
remain the public entry points. This is an ordinary command chooser in the
application layout, not a second registry or an OCaml callback inside native paint.

Embedded presentation uses its parent's width and the ordinary style API, with
the appearance's bounded visible-row sizing. It has no viewport backdrop,
deferred popup, focus trap, mount-time autofocus or focus restoration. Tab visits
header/query/footer controls and surrounding application controls normally. The
query is native-owned; application state changes retain editing, selection and
undo. Explicit Focus still works when the palette is eligible. A higher modal
blocks its input; hiding/disabling retains state without permitting interaction.

Enter or clicking an enabled row invokes its registry command and leaves the
chooser/query/selection mounted. It does not emit `Dismissal.Selected` in this
mode. Native editing actions use ordinary current/last eligible document routing;
the embedded query is not a document editing target. Such an action may return
focus to its document. Application callbacks do not implicitly move focus.

Escape belongs to the focused query or nonsearchable palette frame. Marked text
is canceled first; `Clear_query_first` clears nonempty text next. Otherwise the
existing `on_dismiss Escape` callback is a cancellation request: embedded native
state remains mounted, and the application chooses whether to remove it. Ordinary
header/footer child Escape propagation is preserved. Outside clicks do not
dismiss embedded content. The modal dismissal contract remains unchanged.

Search policies, loading, rich content, grouped rows and query-fenced external
publication retain their existing bounds and lifetime contracts. Live changes of
presentation retain query identity while recalculating focus policy. Becoming
modal captures the current/last eligible document target before entering the
trap. Outside focus at that transition becomes the return target; if focus
already belongs to the chooser, its previous outside return target is preserved.
A spent modal session remains closed until remounted; changing presentation
does not resurrect it. Unmount retires resources and observer identity in either mode.

The unpublished paired palette-options representation appends presentation tag
0 (Modal) or 1 (Embedded). Default options remain elided. Both runtimes require
the same revision. Independent bytes, unknown tags, focus coexistence, repeated
activation, visibility/modal gates, presentation changes and actual installed
gallery interaction have local evidence; current-source hosted and broader
platform/release qualification remain separate.
