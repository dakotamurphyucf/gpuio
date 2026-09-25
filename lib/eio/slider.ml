open Core
module Bonsai = Bonsai.Cont
module S = Gpuio.Slider

type observation =
  | Native of S.Snapshot.t
  | Reply of S.Snapshot.t * S.Snapshot.t

type t =
  { window : App.Window.t
  ; controller : Gpuio.Key.t
  ; config : S.Config.t
  ; initial : S.Value.t
  ; snapshot : S.Snapshot.t option
  ; observe : observation -> unit Bonsai.Effect.t
  ; on_event : S.Event.t -> unit Bonsai.Effect.t
  }

let same_lease left right =
  Gpuio_protocol.Window_id.equal (S.Expert.window left) (S.Expert.window right)
  && Gpuio_protocol.Node_id.equal (S.Expert.node left) (S.Expert.node right)
;;

let create window ~config ~initial ?on_event graph =
  let snapshot, observe =
    Bonsai.state_machine0
      ~default_model:None
      ~equal:(Option.equal S.Snapshot.equal)
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
               && S.Revision.compare
                    (S.Snapshot.revision previous)
                    (S.Snapshot.revision next)
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
      | S.Event.Observed snapshot
      | Drag_started snapshot
      | Preview snapshot
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
  Gpuio.View.slider
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
  let%bind result = App.Window.Expert.slider_command t.window expected command in
  let%map () =
    match result with
    | Ok next -> t.observe (Reply (expected, next))
    | Error _ -> Bonsai.Effect.Ignore
  in
  result
;;

let command t command =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error S.Command_error.Not_mounted)
  | Some expected -> send t expected command
;;

let read_snapshot t = command t Read_snapshot
let focus t thumb = command t (Focus thumb)
let cancel_drag t = command t Cancel_drag
let replace t ?if_revision value = command t (Replace { value; if_revision })

let replace_if_unchanged t expected value =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error S.Command_error.Not_mounted)
  | Some current when not (same_lease current expected) ->
    Bonsai.Effect.return (Error S.Command_error.Stale_slider)
  | Some _ ->
    send t expected (Replace { value; if_revision = Some (S.Snapshot.revision expected) })
;;
