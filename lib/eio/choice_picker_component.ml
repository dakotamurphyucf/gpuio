open Core
module Bonsai = Bonsai.Cont
module P = Gpuio.Choice_picker
module Input = Gpuio.Text_input

type t =
  { editor : Editor_controller.t
  ; config : P.Config.t
  ; initial_text : string
  ; on_event : P.Event.t -> unit Bonsai.Effect.t
  }

let create command ~config ?(initial_text = "") ~on_event graph =
  Input.validate_text ~mode:Single_line initial_text |> Or_error.ok_exn;
  let editor = Editor_controller.create_with_command command graph in
  let open Bonsai.Let_syntax in
  let%arr editor = editor
  and config = config
  and on_event = on_event in
  let observed event =
    let snapshot =
      match event with
      | P.Event.Query_changed snapshot -> Some snapshot
      | Selection_requested request -> P.Selection_request.query request
      | Open_requested _ | Visibility _ -> None
    in
    match snapshot with
    | None -> on_event event
    | Some snapshot ->
      Bonsai.Effect.Many [ Editor_controller.observe editor snapshot; on_event event ]
  in
  { editor; config; initial_text; on_event = observed }
;;

let view ?key ?style ?appearance ?trigger ?empty ?footer ?groups ?options t =
  let open Or_error.Let_syntax in
  let%bind query =
    match P.Config.search t.config with
    | None -> Ok None
    | Substring | Application ->
      P.Query.create
        ~controller:(Editor_controller.key t.editor)
        ~initial_text:t.initial_text
        ()
      |> Or_error.map ~f:Option.some
  in
  let%bind description =
    P.Description.create
      ~config:t.config
      ?appearance
      ?query
      ?trigger
      ?empty
      ?footer
      ?groups
      ?options
      ()
  in
  let key = Option.value key ~default:(Editor_controller.key t.editor) in
  Gpuio.View.choice_picker ~key ?style ~on_event:t.on_event description
;;

let snapshot t =
  match P.Config.search t.config with
  | None -> None
  | Substring | Application -> Editor_controller.snapshot t.editor
;;

let command t command =
  match P.Config.search t.config with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Substring | Application -> Editor_controller.command t.editor command
;;

let focus t = command t Input.Command.Focus

let replace t ?if_revision ~selection ~undo text =
  command t (Input.Command.Replace { text; selection; undo; if_revision })
;;

let replace_if_unchanged t request text =
  match P.Config.search t.config, P.Selection_request.query request with
  | None, _ | _, None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | (Substring | Application), Some snapshot ->
    Editor_controller.replace_if_unchanged
      t.editor
      snapshot
      ~selection:End
      ~undo:Record
      text
;;
