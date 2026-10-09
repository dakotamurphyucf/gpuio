# Inspect native read-only selection without the clipboard

[window_selection_preview.ml](window_selection_preview.ml) and its
[interface](window_selection_preview.mli) demonstrate correlated `App.Window`
selection APIs on an exact window handle. [Runtime_page](runtime_page.ml) mounts
it. `B = Bonsai.Cont` owns one local notice; `E = Bonsai.Effect` sequences native
replies; `V` describes views. This selection is read-only text/document selection,
not editable input values, formatting state or a selectable-list selected set.

After [setup](../../docs/development.md), from the root:

```sh
GPUIO_JOBS=2 ./scripts/gpuio exec dune build examples/gallery/main.exe -j 2
./scripts/gpuio exec dune exec examples/gallery/main.exe
python3 scripts/test_gallery.py --section runtime
```

Choose Runtime & windows and drag across the three selectable fixture lines.
Use Primary+Shift+U to inspect, E to end drag, Y to check, K to clear. Primary is
platform-adapted; it is Command on macOS. No data provider/file is needed.
[README](README.md) records authored native checks/platform limits, not acceptance
from compilation. No build/native command was newly executed here.

## Notice state and typed request results

`B.state` supplies notice/setter; `let%arr` combines palette/notice/setter into current
view and effects. inspect awaits `App.Window.selected_text` and formats UTF-8 byte
length with escaped OCaml %S. Empty string reports no selection. check asks
`has_text_selection` without retrieving text. action handles unit Result for clear
and end_drag; Window.Error is shown as typed S-expression. Constructing these
effects does not read selection until they are handled.

[Window contracts](../../lib/eio/app.mli) restrict collection to registered read-only
renderers in active native scope. `selected_text` joins nonempty fragments in logical
order with newlines, default max 65536 bytes; oversized results error rather than
truncate (configurable public bound 0–262144). This preview uses the default. Byte
length differs from Unicode characters/graphemes, as café/京都/👩‍💻 demonstrate.
None of these operations reads password/OTP/text-input contents or clipboard data.

Clear acts on current native window geometry and renderer-local ranges at execution,
including inactive registrations. End stops selection drag/auto-scroll while
preserving its visible range. They are distinct actions; ending a drag is not
clearing/copying text. Exact window generation/correlated request ownership means
closing the window reports typed failure rather than routing to a recycled slot.
There is no polling subscription or Eio producer task in this component.

## Commands and interaction trace

command creates stable gallery.selection.* IDs with label, Primary+Shift shortcut
and Override priority; `on_invoke` returns the prebuilt deferred operation. A local
`Command.Registry` is installed through `command_scope`. Buttons expose Check/Clear;
Inspect/End are available by shortcuts. The text column has User_select=true and
three normal text nodes, permitting ranges across nodes. Notice is ordinary text,
not editable selection state or an application-stored range.

Trace: native drag selects Unicode fragments → Inspect shortcut resolves command →
`selected_text` request observes current registered ranges → reply returns whole
UTF-8 fragments/newline joins → setter changes notice → `let%arr` rebuilds it. End
then stops drag while keeping range; Check remains true until Clear or another
native selection change. Clipboard is never touched by this sequence.

For another selection viewer, retain byte bounds/error handling and use `selected_text`
rather than simulated Copy/paste. Do not conflate window read-only ranges with a
native editor's selection/IME ownership or assume the notice remains current after
later selection changes. Physical drag/shortcut and native geometry behavior need
their actual runtime harness; source review does not establish them.
