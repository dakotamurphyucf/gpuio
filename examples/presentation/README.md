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
avatar asset fallback, loading indicators, rating and wider scale/content checks
remain to be implemented and added here. OCH-46 separately integrates the finished
milestone into the polished agent-chat showcase.
