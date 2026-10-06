# Keep window-local feedback and native note drafts

[review_feedback.ml](review_feedback.ml) and [review_feedback.mli](review_feedback.mli)
combine a controlled rating, private native note editor, guidance accordion and
interactive contributor preview. Everything is session-local: no feedback is
persisted or sent to a provider. The contributor and run are simulations.

Build/run from the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Review feedback. Adjust rating with arrow keys, Clear it, open Private review
notes and type, collapse/reopen, then change guidance expansion mode. Hover/focus
About this contributor and follow Open contributor sources. macOS is the v1
native target; Linux GUI checks are [informational](../../../docs/platform-release-policy.md).

## A pure reducer inside one retained Bonsai graph

Read `Guidance`, `Model.initial`, `Action`, `apply` and `component`. Guidance is a
closed variant Sources/Changes/Verification, with labels and explanatory text.
It builds validated `Disclosure.Id`/Choice values for Check the sources, Inspect
the changes and Verify the outcome. These IDs are application item identities,
not file paths or asynchronous tasks.

Initial model has Rating.Config value 0 (unrated, default maximum 5), empty note,
notes collapsed, Single guidance mode with allow_empty and no expanded items,
and preview false. Actions are Rate, Note, Toggle_note, Guidance, Guidance_mode
and Preview. The pure `apply` function updates only the corresponding field;
Rating.Config.apply_request and Disclosure.apply_request/with_mode own their
validated request semantics.

`Bonsai.Cont.state_machine0` installs that reducer with `default_model` and
returns a reactive model plus an injection function. `inject action` is a deferred
Bonsai effect: ordered action application happens against the latest model,
not a stale value captured in a displayed control. `let%arr` combines current
portrait/model/inject/theme values into the view. No worker task, Eio delay or
network scope is added here.

## Native rating and note ownership

`View.rating ~config:model.rating ~on_request` queues Rate requests. The
[rating contract](../../../lib/core/rating.mli) applies Increase/Decrease against
the latest value, saturates at zero/maximum and treats zero as unrated. Hover
preview changes no application rating. Clear injects Request.clear, and status
reflects Not rated yet or N of 5. The visible arrow shortcut label displays a
hint; it does not register a separate command binding.

`Gpuio.View.text_input` uses stable controller run-feedback-note, multiline
configuration, label Private review note, three-to-six rows and initial_text
from the last application snapshot. Native text state/composition/selection/undo
belong to the editor. Changed snapshots and Submitted captures inject Note with
exact text; Search_changed is ignored. Changing initial_text during ordinary
updates does not overwrite an existing live native draft; it seeds a new mount.
The [text input interface](../../../lib/core/text_input.mli) defines those roles.

Private review notes uses `View.disclosure ~hidden:Retain`. Toggling updates only
`note_open`, and native collapse retains that live editor rather than discarding
its selection/undo for a visual hide. Route/page unmount is different: native
content can retire, while the still-active Bonsai graph keeps the latest note
snapshot and seeds a remount. There is no storage service behind this snapshot.

## Guidance and contributor preview

`View.accordion` gets the accepted Disclosure model and queues Guidance requests.
One section selects Single with allow_empty; Keep sections open selects Multiple.
Mode changes reconcile expanded items through `Disclosure.with_mode`. Pure
content maps current IDs back to Guidance descriptions. `~hidden:Unmount`
removes inactive guidance content; it is safe because those panels have no
editors/tasks to retain. Their model expansion preference remains in Bonsai.
Read the [disclosure contract](../../../lib/core/disclosure.mli).

`Contributor_portrait.component` supplies the portrait view under the application/
window capabilities; [contributor_portrait.ml](contributor_portrait.ml) owns local
image registration/decoding observations separately. The surrounding
`View.hover_card` has controlled open state `model.preview`, stable key
review-contributor and a 290-pixel About the local assistant configuration.
Native hover/focus/Escape requests call Preview through on_open_change; its
anchor's click effect is Ignore because normal hover-card behavior owns opening.
The content has portrait, simulation text, source link and Close button.

`Bonsai.Edge.on_change active` injects Preview false when the page becomes
inactive, preventing an open transient preview from following navigation.
It does not clear rating, note or guidance. Open contributor sources combines
closing preview with the supplied `on_sources` navigation effect. No URL is
fetched and no external browser/navigation is launched by this link.

## Trace feedback across page changes

Typing in the expanded native note produces a Changed snapshot; the queued
handler injects Note; the reducer stores text; `let%arr` derives labels/views.
Collapsing changes note_open and native visibility while retaining that editor.
Leaving Feedback closes the controlled hover preview. Inspector constructs this
component outside its route selection, so graph state survives route changes/
inspector closure even when native page content unmounts. Returning remounts
from the stored snapshot; another window has a separate reducer/editor.
Window closure ends feedback and portrait resources under their own normal
window/application ownership.

Rating arrow input follows the same event → ordered request → reducer → derived
controlled config path. Selecting an accordion section updates the disclosure
model, not native source data. Request acceptance/view derivation does not prove
a physically painted frame or external OS keyboard/accessibility acceptance.

A small adaptation is another pure guidance section: extend Guidance.all/name/
description with a unique ID and retain the explicit lookup. For real submitted
feedback, add a separate service/effect and visible success/error model; local
rating state is not a sent report. Preserve stable editor identity and distinguish
retained collapse from page remount when adding more editable content. The
[README](../README.md) lists `python3 scripts/test_agent_chat_feedback.py` and its
macOS prerequisites; [existing evidence](../../../docs/evidence/agent-chat-m5.md)
records actual native/focus/retention checks. This prose review runs none of them.
