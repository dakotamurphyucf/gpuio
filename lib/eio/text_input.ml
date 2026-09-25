open Core
module Bonsai = Bonsai.Cont
module Input = Gpuio.Text_input

type t =
  { editor : Editor_controller.t
  ; config : Input.Config.t
  ; initial_text : string
  ; on_submit : Input.Submission.t -> unit Bonsai.Effect.t
  ; on_event : Input.Event.t -> unit Bonsai.Effect.t
  }

let create window ~config ?(initial_text = "") ?on_submit graph =
  let editor = Editor_controller.create window graph in
  let on_submit =
    Option.value on_submit ~default:(Bonsai.return (fun _ -> Bonsai.Effect.Ignore))
  in
  let open Bonsai.Let_syntax in
  let%arr editor = editor
  and config = config
  and on_submit = on_submit in
  Input.validate_text ~mode:(Input.Config.mode config) initial_text |> Or_error.ok_exn;
  let on_event = function
    | Input.Event.Changed snapshot -> Editor_controller.observe editor snapshot
    | Submitted submission ->
      Bonsai.Effect.Many
        [ Editor_controller.observe editor (Input.Expert.submission_snapshot submission)
        ; on_submit submission
        ]
  in
  { editor; config; initial_text; on_event; on_submit }
;;

let view ?style t =
  Gpuio.View.text_input
    ?style
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
let select t selection = Editor_controller.select t.editor selection

let replace t ?if_revision ~selection ~undo text =
  Editor_controller.replace t.editor ?if_revision ~selection ~undo text
;;

let clear_if_unchanged t submission =
  Editor_controller.replace_if_unchanged
    t.editor
    (Input.Expert.submission_snapshot submission)
    ~selection:Start
    ~undo:Record
    ""
;;

let submit t =
  let open Bonsai.Effect.Let_syntax in
  let%bind result = command t Submit in
  match result with
  | Error error -> Bonsai.Effect.return (Error error)
  | Ok snapshot ->
    (match Input.Expert.submission snapshot with
     | Error _ -> Bonsai.Effect.return (Error Input.Command_error.Composing)
     | Ok submission ->
       let%map () = t.on_submit submission in
       Ok ())
;;

let read_snapshot t = command t Read_snapshot
