# Message follow presentation

OCH-41 composition contract, based on the pinned styled MessageScroller.
This uses the existing public managed-list, style and native animation APIs.
The list remains the sole scroll owner; no second offset, timer or OCaml frame
loop is introduced. The reusable gallery recipe lives in
`examples/gallery/model/message_follow.ml`.

Show the jump control and optional bottom fade only after a matching viewport
reports neither tail following nor `at_end`. Before the first observation, or
while a short/empty list is at its end, both remain invisible. A paused follow
mode alone does not imply unread content. Appending and streaming use the
existing collection edits and native tail policy.

The viewport retains its key and bounded geometry. Overlays use absolute
positioning, so changing visibility cannot move messages, change their width or
add a gap below the list. The fade is decorative and pointer-transparent. Its
right inset reserves space for the vertical scrollbar. The jump button keeps
its caller-supplied native action, label and appearance.

Two retained animation wrappers tween opacity for 200 ms; the button slides into a 48-pixel-high overlay slot, from below the clipped
viewport to a 16-pixel bottom inset. The caller supplies content that fits that
slot. The gallery supplies a 44-pixel action with a 20-pixel line height, so
Large typography does not clip the ordinary scaled toolbar padding. A settled hidden slot is entirely outside the clip, so its inert hitbox
cannot block messages underneath. This is a presentation difference from the
pinned widget's eight-pixel slide. Updates retarget from native painted values. No
initial animation runs on mount. Reduced motion follows the existing application
preference; disabling animation uses zero duration. Settled wrappers request no
more frames, and unmount/hidden-window behavior belongs to the native scheduler.

As soon as the jump target becomes hidden, its child subtree becomes `Inert`:
it leaves native input, Tab order and accessibility while the outer wrapper may
finish fading. Applying `Inert` to the animation ancestor would suppress that
exit, so the gate belongs below the wrapper. Disabled controls alone are not an
adequate substitute for removing an invisible control from accessibility.

The Collections gallery keeps its explicit toolbar action for comparison and
adds independent jump, fade and motion switches. A separate overlay action uses
the same generation-bound `Controller.jump_to_latest`; it does not splice rows or
reset native list state. Log semantics use explicit `Live.Off` for fragment
streaming. This presentation adds no announcement batching or history mirror.

Validate transition targets, retained list/control identities, lack of scroll
commands on presentation updates, inert input during exit, actual overlay bounds,
native settlement and teardown. Physical scroll/focus/VoiceOver/GPU coverage must
be recorded separately from source or simulated layout tests.
