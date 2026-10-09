# OTP caret lifetime

The OTP editor uses a native 500 ms caret phase. Focus and accepted editing or
selection changes show the caret immediately and restart the full visible phase.
Unrelated redraws and appearance changes do not restart the phase. No application
callback or editor revision is produced by blinking.

A single cancellable timer belongs to the retained native editor. It runs only
while that editor is focused, editable, visible within its window/content clip,
allowed by the current input route and modal gate, and in an active window, with
a collapsed selection and no composition. Reduced motion uses a steady caret.
Read-only and provisional composition also retain steady caret presentation.
Selection geometry remains available while the painted caret is off.

Blur, hiding, disabling, removal and window deactivation cancel the timer.
Clipping and reduced-motion changes cancel it during the next layout; a pending
deadline rechecks eligibility before changing paint state or rescheduling. When
eligible again, the editor begins a full visible phase. No idle-window polling,
detached task, strong editor capture or OCaml clock dependency is introduced.
Timer identity prevents a canceled phase from affecting a newer editing session.

This is the chosen implementation contract; verification belongs in the evidence
and catalog records. Fake-clock/TestPlatform checks cannot establish physical
macOS visual, input-method or accessibility acceptance.
