# Navigation instances, history and retained pages

[Source](journey_preview.ml) · [Interface](journey_preview.mli) · [Gallery](README.md)

`Journey_preview.component` is the navigation-history card on **Carousels &
journeys**. The [parent page](journeys_page.md) composes it with independent
carousels and a sidebar. Changing the sidebar does not change this history.

## Follow the pure model first

`Chapter.t` is a route payload: Imagine, Shape or Share. `Chapter.next` cycles
through them. A payload is different from a navigation instance: visiting Imagine
again must create another `Navigation_stack.Id`, not reuse the root's ID.

`Model.t` contains immutable `history`, the root's ID, a serial for new identities,
the hidden-page policy, motion choice and any admission error. `Model.entry`
combines the serial and chapter name into a validated ID and creates a labelled
entry. `Model.reset` creates three entries with Imagine current and Shape/Share
in the forward branch. Each reset uses a fresh serial, so it also replaces native
page identities. The serial belongs to this component; it is not a persisted ID
or a native handle.

`Model.apply` handles these explicit actions:

- Back and Forward move through existing history. Root returns to the first entry
  while preserving the forward branch.
- Push visits the next chapter with a new ID and discards the old forward branch.
  Returning to the same chapter later still creates a different instance.
- Replace installs a new Share instance at the current position, preserving the
  other entries. Its button is disabled at the root so this example keeps a root
  editor to return to.
- Reset creates a fresh initial history but preserves motion/retention preferences.
- Toggle_retention and Cycle_motion change presentation policy without changing
  the route history.

`Model.install` handles `Or_error` from push/replace. A rejected history operation
keeps the current model and shows the error; it does not crash the example when
history reaches the library's bound. `ok` unwraps only statically valid fixture
values. `Motion.config` maps Slide to the default native slide, Fade to 200 ms,
and Immediate to no transition. These are GPUIO policies, not OCaml animation loops.

## Connect the model to Bonsai and GPUIO

`B` is `Bonsai.Cont`. `B.state_machine0` returns a reactive model and the `act`
injector. Calling `act Push` constructs a Bonsai effect; the reducer runs when a
button executes that effect. It receives the latest model. `let%arr` reads the
model, palette, injector and editor output to construct the current `View` tree.
It performs no I/O and does not mutate native controls during graph evaluation.

`Editor.create` constructs one controller for the root's single-line **Journey
note**. Its initial text is a seed for a native editing session, not a controlled
rewrite on every render. `V.navigation_stack` receives the history, motion and
hidden policy; its `content` callback describes each instance with a heading,
identity caption and, only for the root ID, that editor's view. A later Imagine
visit deliberately has no editor. Do not reuse this one editor view across every
entry; an application needing independent editors should create separately keyed
controllers for those entries.

Under Retain, hiding the root makes its native content inert to input and
accessibility but preserves its buffer. Under Unmount, leaving it destroys that
native session; returning seeds a fresh buffer. Retention is not persistence.
Removing this entire gallery page also retires its Bonsai/controller state.
Tasks owned by an application/conversation Eio scope would have a separate
lifetime; this example starts no background task.

## Trace an interaction

Type `a` into Journey note. Continue journey executes `act Forward`, moves the
current instance to Shape, and removes the root editor from accessible input.
Journey root executes `pop_to_root`; with Retain selected, the note still reads
`a`. Visit next chapter now pushes a *new* Shape instance and removes the old
Shape/Share forward branch. Replace with Share changes that instance while
keeping the root. The history caption reports back/forward counts; the visible
page reports its instance ID.

Turn off Retain journey pages, leave the root and return. The buffer now has its
initial text. Cycle motion to see the same history operations with different
native transition policies. The controls and semantics should remain usable with
reduced motion; this walkthrough does not establish per-frame animation quality.

## Run and validate

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build -j2 examples/gallery/main.exe
_build/default/examples/gallery/main.exe
python3 scripts/test_gallery.py --section journey-history --images scratch/journey-history
```

The bounded macOS fixture uses actual Space input plus native accessibility
button actions. It checks both themes and all three motion modes, root guards,
forward-branch replacement, repeated route visits, retained/unmounted drafts and
page remount. It records each case and screenshots. Those samples are not
VoiceOver, IME, every-frame motion, performance or Linux desktop acceptance.
