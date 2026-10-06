# Register a local portrait and exercise real decode fallback

[contributor_portrait.ml](contributor_portrait.ml) and
[contributor_portrait.mli](contributor_portrait.mli) supply the About this
contributor preview's avatar. Initials GP appear initially; Local portrait
publishes a valid embedded SVG, while Unavailable portrait publishes malformed
local PNM bytes that native decoding rejects. This distinguishes registration
success from image decoding success. No files, URLs or external portraits are
fetched.

Build/run from the repository root after [isolated setup](../../../docs/development.md):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/agent_chat/main.exe -j 2
_build/default/examples/agent_chat/main.exe
```

Open Review feedback, hover/focus About this contributor, choose Local portrait,
then Unavailable portrait, then Local again. macOS is the v1 target; Linux GUI
qualification is [informational](../../../docs/platform-release-policy.md).

## Resource state differs from image state

Read `Resources`, `Mode`, `initialize` and `component`. Resources is
Absent | Pending | Ready {portrait; unavailable} | Failed. It tracks registration,
not a selected image's decode status. Mode is Initials | Portrait | Unavailable;
component starts Initials and a separate observed `Image.State.Loading`.
The Resources variable is created during graph construction, not inside its
`let%arr` rendering. `B.state` returns reactive mode/image observation and setter
effects; `B.Expert.Var.value` exposes resource observation.

`initialize` is a deferred effect. Its first thunk changes Absent/Failed to
Pending, while Pending/Ready ignore repeated initialization requests. It
registers a small circle/star SVG in `App.Window.scope window`, then registers
format Pnm bytes Deliberately unavailable local portrait. `Asset.Source.of_bytes`
and [Asset.register](../../../lib/eio/asset.mli) deal with immutable encoded bytes;
publishing malformed image bytes can succeed because pixel decoding is separate.

If the first registration fails, resource state becomes Failed. If the second
fails, it releases the first registration before setting Failed. When both
succeed, it stores their handles in Ready. Successful repeated toggles reuse
these two registrations; a failed attempt can retry through the same explicit
controls. Window scope cancellation owns disposal and suppresses late allocation
completion, so hiding the hover card does not release needed assets prematurely.

## Follow mode selection and asynchronous observation

`let%arr` combines resource/mode/observed/theme values into presentation. Asset
selection is Some portrait only for Ready+Portrait, Some malformed handle only
for Ready+Unavailable, otherwise None. An optional handle is not an indication
that decoding has succeeded.

Each button combines `set_mode`, `set_observed Loading` and `initialize` effects.
The selected mode's button is disabled except when registration Failed, allowing
a retry. There is no Initials button after startup; decode failure automatically
uses the native fallback. `View.avatar ~on_change:set_observed` gets the selected
asset, explicit fallback GP and meaningful description GPUIO local assistant.
It uses 36 × 36 size, accent colors and 13-pixel fallback font.

A Local portrait click therefore sets mode/loading, publishes resources if needed,
derives the avatar asset, and native decoding later queues Image.State.Ready.
The setter effect updates observed state and the derived status says Local
portrait ready. Unavailable portrait instead queues Failed, paints GP fallback
natively and says Portrait unavailable · showing initials. Registration Failed
has a different retry message. In Initials mode, the status says Contributor
initials; no asset means the [avatar contract](../../../lib/core/avatar.mli)
creates no artificial image failure observation.

Source replacement invalidates old native image observations, preserving one
avatar view identity while switching its asset/fallback. `observed` snapshots
are not decoded pixels stored in Bonsai. Frame updates do not call OCaml to draw
the avatar, and encoded publication/window acknowledgement is not physical paint
or decode completion. [Review_feedback](review_feedback.md) owns controlled
hover-card visibility and closes it on inactive routes while keeping the graph/
window-local state. Another window has independent registrations/mode.

A small adaptation is another embedded SVG portrait: keep explicit Source format,
valid bytes, accessible description and window scope. Add a Mode/resource handle
if it is a distinct fixture; do not register anew on each toggle/render. Real
network portraits need an explicit service/cancellation policy before bytes are
registered. Keep decode errors separate from fetch/registration errors.
The optional `python3 scripts/test_agent_chat_tour.py` covers local decode/fallback
under the [README](../README.md)'s macOS prerequisites;
[existing evidence](../../../docs/evidence/agent-chat-m5.md) records acceptance.
This documentation review performs no native decode or hover/focus test.
