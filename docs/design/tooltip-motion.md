# Tooltip entry and rapid switching

OCH-41. Native implementation and focused regressions pass; full qualification
remains open. See [evidence](../evidence/tooltip-motion-och41.md).

`Tooltip.Config.create ?motion` accepts `Tooltip.Motion.Immediate` (the default)
or `Enter_and_switch`. Motion runs in Rust inside the actual deferred popup.
Bonsai and ordinary `View.tooltip` use the same configuration.

An ordinary opening fades from zero to full opacity and translates upward from
four logical pixels below its settled position over 150ms, using cubic ease-out.
A switch from a recently closed, actually painted animated tip on the same trigger
row slides horizontally over 200ms using cubic ease-in-out, without fading.
Trigger origins must differ by less than ten logical pixels vertically; the
horizontal displacement uses the trigger centers, including different widths.
A recent cross-row switch presents immediately. Expired history uses normal entry.
The receiving tooltip's existing `skip_delay` bounds the positional history.
These are explicit presentation rules, not new geometry values in the bridge.

Animated managed tips skip the ordinary show delay when another animated managed
tip is visible. A native request to open one closes other animated managed tips
first, publishing their false notifications before the new true notification.
Only visible peers participate; hidden or modal-blocked accepted state is retained.
All replacements share one visibility/focus synchronization, including multiple
explicitly opened tips.
The displaced tip is suppressed while its previous hover/focus interest remains,
so an old focus notification cannot reopen it immediately. Hidden content remains
retained under the existing policy: closing a tip does not dispose its model or
change its child IDs. Removing the tooltip still disposes its content normally.

Controlled tips remain application-owned. An open/close request only emits the
usual intent; it neither paints unaccepted content nor forcibly closes other
controlled tips. Accepted application swaps can use the same positional history.
Explicit initial-open configuration remains accepted state. Immediate tips and
hover cards keep their existing timing and coexistence behavior. Hover cards do
not expose this motion option: their pinned styled render path is unanimated.

The pinned upstream provider shares one visible tooltip and records trigger
bounds while replacing it. GPUIO keeps its existing retained and controlled
content contract and uses a bounded previous-value snapshot during accepted
closure/replacement. This also permits a smooth same-row switch across a short
accepted close/open gap. It does not retain an outgoing visual or native editor.

A stable native frame identity survives style/motion configuration changes.
First actual paint starts the animation; repeated layout does not consume entry
time. Reduced motion or disabling the effect permanently settles the current
visible opening. Re-enabling does not replay it. Hiding/removal retires frame
state; reopening has fresh entry. Motion-only changes leave pending native show/
hide deadlines and accepted visibility intact. Active motion requests frames only
after paint, with no new timer, polling loop or per-frame OCaml callback.

Per-window history stores only an owner ID, painted trigger bounds and close time.
Disabled, blocked, hidden or removed owners invalidate it. Unpainted/invisible
openings never supply history. Tooltip admission reserves 512 bytes for fixed
motion/snapshot bookkeeping even in immediate mode; this is not an RSS estimate.
Op92 `Set_tooltip_motion (NodeId, bool)` is valid only for Tooltip nodes. Existing
config, queue limits, generation checks and asynchronous events remain in force.

Current TestPlatform and actual desktop evidence will be recorded separately.
Physical macOS keyboard/IME/VoiceOver, visual and resource qualification remains
required for release; compiling an example does not provide that evidence.
