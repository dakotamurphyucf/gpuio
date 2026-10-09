open Core
module Bonsai = Bonsai.Cont
module N = Gpuio.Calendar

type observation =
  | Native of N.Snapshot.t
  | Reply of N.Snapshot.t * N.Snapshot.t

type t =
  { window : App.Window.t
  ; controller : Gpuio.Key.t
  ; config : N.Config.t
  ; initial : N.Selection.t
  ; initial_month : N.Month.t
  ; snapshot : N.Snapshot.t option
  ; observe : observation -> unit Bonsai.Effect.t
  ; on_event : N.Event.t -> unit Bonsai.Effect.t
  }

let same_lease left right =
  Gpuio_protocol.Window_id.equal (N.Expert.window left) (N.Expert.window right)
  && Gpuio_protocol.Node_id.equal (N.Expert.node left) (N.Expert.node right)
;;

let create window ~config ~initial ~initial_month ?on_event graph =
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
      | Selected snapshot
      | Rejected (_, snapshot) -> snapshot
    in
    Bonsai.Effect.Many [ observe (Native snapshot); callback event ]
  in
  { window
  ; controller = Gpuio.Key.of_string_exn controller
  ; config
  ; initial
  ; initial_month
  ; snapshot
  ; observe
  ; on_event
  }
;;

let view ?style ?appearance ?content ?on_viewport_change t =
  Gpuio.View.calendar
    ?on_viewport_change
    ?content
    ?appearance
    ?style
    ~controller:t.controller
    ~config:t.config
    ~initial:t.initial
    ~initial_month:t.initial_month
    ~on_event:t.on_event
    ()
;;

let snapshot t = t.snapshot

let send t expected command =
  let open Bonsai.Effect.Let_syntax in
  let%bind result = App.Window.Expert.calendar_command t.window expected command in
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
let focus_date t date = command t (Focus_date date)
let show_month t month = command t (Show_month month)
let move_months t ~months = command t (Move_months months)
let set_presentation t presentation = command t (Set_presentation presentation)
let replace t ?if_revision selection = command t (Replace { selection; if_revision })
let clear t ?if_revision () = command t (Clear { if_revision })

let replace_if_unchanged t expected selection =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error N.Command_error.Not_mounted)
  | Some current when not (same_lease current expected) ->
    Bonsai.Effect.return (Error N.Command_error.Stale_input)
  | Some _ ->
    send
      t
      expected
      (Replace { selection; if_revision = Some (N.Snapshot.revision expected) })
;;
