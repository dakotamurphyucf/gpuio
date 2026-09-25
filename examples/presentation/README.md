# Component Studio

Public Core/Bonsai/Eio settings and chat-card compositions. Run from the repository:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune exec -j 2 examples/presentation/main.exe
```

Change the appearance, edit the workspace name, show/clear validation, toggle live
responses and use the documentation/result/attachment actions. They update local
demo state; no network, external service or file write is needed. The shortcut
label is explicitly display-only. Native Markdown/code views can occupy the same
message/tool-result slots; document registration remains application-owned.

`Gpuio.Presentation` provides concrete light/dark appearances, typed size/tone/
variant choices and ordinary action-polymorphic views. `Gpuio.Form.field` attaches
validated label/help/error semantics to the supplied native control and composes
its visual layout. The Eio input controller still owns the native editor lease;
changing errors, slots or appearance does not replace it.

Validation (actual macOS windows; all launched child windows are closed):

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune exec -j 2 examples/presentation/main.exe -- --self-test
python3 scripts/test_presentation.py --images scratch/presentation-images
```

The public self-test checks acknowledged frames across theme/error changes and
exact editor snapshot retention. The Python test sends real keyboard input to
its child, checks native field help/error and Link activation, invokes card actions,
checks action geometry, captures both themes and closes the native window.
It requires macOS accessibility access. No Linux desktop behavior is inferred.

This example is the current OCH-33 foundation, not full component acceptance:
the final cross-family content/layout checks remain. OCH-46 separately integrates the finished
milestone into the polished agent-chat showcase.

The Background work group uses `View.loading` with Skeleton, Shimmer and Spinner.
Hide/show preserves their leaf identities; Static/Animate changes only the native
configuration. Application reduced-motion policy also settles them. The native
suite verifies whole-window idle behavior, and the external AX test verifies that
hidden indicators disappear and that indeterminate loading has no numeric value.

The assistant's `View.avatar` uses the existing Eio scoped asset registration.
Use image selects an embedded SVG, Simulate failure selects deliberately invalid
image bytes, and Use initials removes the source. The native leaf chooses its
fallback automatically and preserves the accessible label. Registration success
does not imply successful decoding. Both assets remain scoped to the application;
the example performs no external image fetch. The public self-test and external
AX test cover all three states while keeping the editor intact.

Response feedback uses `View.rating` with one labelled native slider and styled
stars. Click a star to select it, click that selected star again to clear, or use
Right/Up, Left/Down, Home and End. Read-only and disabled controls demonstrate their
different focus/interaction policies. Hover stays native and does not update the
application model.

The example's `rating_action.ml` applies `Rating.Config.apply_request` inside a
Bonsai state machine. This preserves every increment/decrement in an input burst:
requests are evaluated against the latest model, rather than using the value from
an older render. The application may retain its model to reject a request. The
self-test exercises burst saturation, toggling to zero and read-only rejection;
external macOS validation sends actual keyboard input and an AX increment action.
