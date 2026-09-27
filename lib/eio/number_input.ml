open Core
module Bonsai = Bonsai.Cont
module N = Gpuio.Number_input

type observation =
  | Native of N.Snapshot.t
  | Reply of N.Snapshot.t * N.Snapshot.t

type t =
  { window : App.Window.t
  ; controller : Gpuio.Key.t
  ; config : N.Config.t
  ; initial : N.Value.t
  ; snapshot : N.Snapshot.t option
  ; observe : observation -> unit Bonsai.Effect.t
  ; on_event : N.Event.t -> unit Bonsai.Effect.t
  }

let same_lease left right =
  Gpuio_protocol.Window_id.equal (N.Expert.window left) (N.Expert.window right)
  && Gpuio_protocol.Node_id.equal (N.Expert.node left) (N.Expert.node right)
;;

let create window ~config ~initial ?on_event graph =
  let snapshot, observe =
    Bonsai.state_machine0
      ~default_model:None
      ~equal:(Option.equal N.Snapshot.equal)
      ~apply_action:(fun _ previous observation ->
        let next =
          match observation with
          | Native next -> Some next
          | Reply (expected, next) ->
            if Option.exists previous ~f:(fun previous -> same_lease previous expected)
            then Some next
            else None
        in
        match previous, next with
        | _, None -> previous
        | Some previous, Some next
          when same_lease previous next
               && N.Revision.compare
                    (N.Snapshot.revision previous)
                    (N.Snapshot.revision next)
                  > 0 -> Some previous
        | (Some _ | None), Some next -> Some next)
      graph
  in
  let controller = Bonsai.path_id graph in
  let on_event =
    Option.value on_event ~default:(Bonsai.return (fun _ -> Bonsai.Effect.Ignore))
  in
  let open Bonsai.Let_syntax in
  let%arr snapshot = snapshot
  and observe = observe
  and controller = controller
  and config = config
  and callback = on_event in
  let on_event event =
    let snapshot =
      match event with
      | N.Event.Observed snapshot
      | Changed snapshot
      | Rejected (_, snapshot)
      | Committed (_, snapshot)
      | Cancelled (_, snapshot) -> snapshot
    in
    Bonsai.Effect.Many [ observe (Native snapshot); callback event ]
  in
  { window
  ; controller = Gpuio.Key.of_string_exn controller
  ; config
  ; initial
  ; snapshot
  ; observe
  ; on_event
  }
;;

let view ?style t =
  Gpuio.View.number_input
    ?style
    ~controller:t.controller
    ~config:t.config
    ~initial:t.initial
    ~on_event:t.on_event
    ()
;;

let snapshot t = t.snapshot

let send t expected command =
  let open Bonsai.Effect.Let_syntax in
  let%bind result = App.Window.Expert.number_input_command t.window expected command in
  let%map () =
    match result with
    | Ok next -> t.observe (Reply (expected, next))
    | Error _ -> Bonsai.Effect.Ignore
  in
  result
;;

let command t command =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error N.Command_error.Not_mounted)
  | Some expected -> send t expected command
;;

let read_snapshot t = command t Read_snapshot
let focus t = command t Focus
let select t selection = command t (Select selection)
let commit t = command t Commit
let cancel t = command t Cancel
let step t direction = command t (Step direction)
let undo t = command t Undo
let redo t = command t Redo

let replace_draft t ?if_revision ~selection ~undo text =
  command t (Replace_draft { text; selection; undo; if_revision })
;;

let replace_value t ?if_revision ~selection ~undo value =
  command t (Replace_value { value; selection; undo; if_revision })
;;

let replace_value_if_unchanged t expected ~selection ~undo value =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error N.Command_error.Not_mounted)
  | Some current when not (same_lease current expected) ->
    Bonsai.Effect.return (Error N.Command_error.Stale_input)
  | Some _ ->
    send
      t
      expected
      (Replace_value
         { value; selection; undo; if_revision = Some (N.Snapshot.revision expected) })
;;
