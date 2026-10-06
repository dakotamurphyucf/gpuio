# Alert preview: application-owned dismissal and ordinary body actions

[alert_preview.ml](alert_preview.ml) and its [interface](alert_preview.mli) compose
`Presentation.Alert` with controls for semantic variant, size, icon, layout and
visibility. [pages.ml](pages.ml)'s Presentation branch owns the component. Its public
boundary takes a reactive Palette and Bonsai graph and returns a reactive GPUIO view.
It uses no native registration, Eio task, external asset or file/network I/O.

From the repository root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio build examples/gallery/main.exe
./scripts/gpuio exec _build/default/examples/gallery/main.exe
```

Choose **Presentation**, change Alert variant/size/icon, switch Banner, use Alert
action, dismiss and restore. There is no component-specific diagnostic flag or
self-test. [Development](../../docs/development.md) covers prerequisites; this
walkthrough adds no native keyboard, announcement, VoiceOver or platform acceptance.

`component` creates three `B.state_machine0` cycles: Default→Info→Success→Warning→Error,
Medium→Large→XS→Small, and local Icon_choice Default→Custom→Hidden. `B.toggle` owns
banner/title/close/disabled/refined/compact; title and close start true, the others
false. `B.state` owns visible=true; counters start zero. These are graph state with
injection/setter effects, not mutable fields stored in an alert. `let%arr` reads
current values and palette to derive the view. Effects execute on activation;
constructing `A.create` does not dismiss anything.

`A.create` receives stable key alert, resolved appearance and explicit slots.
Custom icon is an ordinary diamond text View, not a fetched SVG. Body children have
message/actions keys; the message has User_select=true and Unicode text. Optional
title uses `A.title`; Banner omits title by public layout policy even when the title
preference remains true. Refined styles adjust border/radius/body gap; compact
constrains the containing width to 350 instead of 560 logical pixels. Cosmetic
changes preserve surviving body controls.

`A.Close.create` makes a named native button. Its callback returns
`Bonsai.Effect.Many [dismiss (); set_visible false]`: one effect increments the
application counter and another changes visible. The alert does not automatically
change application state on close. Try it: native button activation schedules these
effects, Bonsai derives visible=false and the alert returns an empty hidden root,
retiring its child views/semantics. Restore changes the same state to true and
remounts children; counts are retained. Disable alert close affects only that close
button, not the body's Alert action or Restore control.

Visible alerts carry explicit role Alert, label Alert preview and Live.Off.
Changing variants does not automatically announce them. No background tasks are
retained by the hidden composition; graph choices/counters remain caller-owned.
Read [Presentation.Alert](../../lib/core/presentation.mli) for slot/visibility,
close and live-semantics contracts. To add a custom body action, supply an ordinary
button with its own effect and stable sibling key. For an announcement, explicitly
choose a live policy and validate it natively rather than treating Warning/Error
colors as proof of assistive output.
