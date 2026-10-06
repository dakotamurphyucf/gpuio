# Native palette loading

`Command_palette.Command.Set_loading bool` controls the native palette's loading
presentation through the existing asynchronous `Palette_controller.command` or
`App.Window.Expert.palette_command` API. A new palette starts with loading false.
`Command_palette.Snapshot.loading` observes the applied state. A change advances
the snapshot sequence, but preserves query identity; repeating the same value
does not manufacture a new snapshot sequence.

The loading indicator appears beside the query, or in its own status row when
search is hidden. It uses the existing bounded twelve-stroke native spinner with
a one-second period and a static presentation under reduced motion. The palette
exposes busy semantics and the indicator has a progress role and accessible name.
This is semantic metadata, not a claim of screen-reader qualification.

Loading suppresses both the default empty message and custom empty content.
Suppressed controls lose input/accessibility eligibility immediately, before the
next paint, and their decorative activity is suspended. Existing command rows
remain visible and eligible under normal command rules, matching the pinned
source's loading policy. Header/footer content remains available. Loading never
invokes, dismisses or disables a command by itself.

Changing this presentation does not edit the query, selection, undo history or
marked text. It is allowed during composition and while another overlay owns
interaction. Query, focus and highlight commands retain their stronger interaction
and composition checks. Closing/removing the palette still invalidates commands.
Hidden/closed palettes do not mount the spinner; clearing loading removes it.
Previously queued frame callbacks may finish, but must not renew its animation.

For delayed work, use `command_if_query_unchanged ~expected (Set_loading false)`
so completion for an older query cannot clear the current query's loading state.
Native admission reads the current editor revision, including unobserved edits.
The query check does not distinguish two jobs for the same query: applications
must also cancel or fence superseded jobs with their own request identity.
This API starts no search and does not publish results. Atomic external-result
admission and persistent embedding remain separate required work.

The unpublished exact protocol epoch 3 adds command tag 4 `Set_loading` and
appends a Boolean to palette snapshots in events 79 and 80. This is an in-repository
paired representation change, not backwards-compatible decoding of old snapshot
bytes. OCaml and Rust packages must be built from the same release revision.
