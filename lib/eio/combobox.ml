open Core
module Bonsai = Bonsai.Cont
module Combo = Gpuio.Combobox
module Input = Gpuio.Text_input

type t =
  { editor : Editor_controller.t
  ; config : Combo.Config.t
  ; initial_text : string
  ; on_event : Combo.Event.t -> unit Bonsai.Effect.t
  }

let create window ~config ?(initial_text = "") ~on_select graph =
  Input.validate_text ~mode:Single_line initial_text |> Or_error.ok_exn;
  let editor = Editor_controller.create window graph in
  let open Bonsai.Let_syntax in
  let%arr editor = editor
  and config = config
  and on_select = on_select in
  let on_event = function
    | Combo.Event.Changed snapshot -> Editor_controller.observe editor snapshot
    | Selected selection ->
      Bonsai.Effect.Many
        [ Editor_controller.observe editor (Combo.Selection.snapshot selection)
        ; on_select selection
        ]
  in
  { editor; config; initial_text; on_event }
;;

let view ?style ?appearance t =
  Gpuio.View.combobox
    ?style
    ?appearance
    ~initial_text:t.initial_text
    ~controller:(Editor_controller.key t.editor)
    ~config:t.config
    ~on_event:t.on_event
    ()
  |> Or_error.ok_exn
;;

let snapshot t = Editor_controller.snapshot t.editor
let command t command = Editor_controller.command t.editor command
let focus t = Editor_controller.focus t.editor

let replace t ?if_revision ~selection ~undo text =
  Editor_controller.replace t.editor ?if_revision ~selection ~undo text
;;

let replace_if_unchanged t selected text =
  Editor_controller.replace_if_unchanged
    t.editor
    (Combo.Selection.snapshot selected)
    ~selection:End
    ~undo:Record
    text
;;
