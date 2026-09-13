open Core
module Bonsai = Bonsai.Cont
module Input = Gpuio.Text_input

type observation =
  | Native of Input.Snapshot.t
  | Reply of Input.Snapshot.t * Input.Snapshot.t

type t =
  { window : App.Window.t
  ; controller : Gpuio.Key.t
  ; snapshot : Input.Snapshot.t option
  ; observe : observation -> unit Bonsai.Effect.t
  }

let same_lease left right =
  Gpuio_protocol.Window_id.equal (Input.Expert.window left) (Input.Expert.window right)
  && Gpuio_protocol.Node_id.equal (Input.Expert.node left) (Input.Expert.node right)
;;

let create window graph =
  let snapshot, observe =
    Bonsai.state_machine0
      ~default_model:None
      ~equal:(Option.equal Input.Snapshot.equal)
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
               && Input.Revision.compare
                    (Input.Snapshot.revision previous)
                    (Input.Snapshot.revision next)
                  > 0 -> Some previous
        | (Some _ | None), Some next -> Some next)
      graph
  in
  let controller = Bonsai.path_id graph in
  let open Bonsai.Let_syntax in
  let%arr snapshot = snapshot
  and observe = observe
  and controller = controller in
  { window
  ; controller = Gpuio.Key.of_string controller |> Or_error.ok_exn
  ; snapshot
  ; observe
  }
;;

let key t = t.controller
let observe t snapshot = t.observe (Native snapshot)
let snapshot t = t.snapshot

let send t snapshot command =
  let open Bonsai.Effect.Let_syntax in
  let%bind result = App.Window.Expert.editor_command t.window snapshot command in
  let%map () =
    match result with
    | Ok next -> t.observe (Reply (snapshot, next))
    | Error _ -> Bonsai.Effect.Ignore
  in
  result
;;

let command t command =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some snapshot -> send t snapshot command
;;

let focus t = command t Focus
let select t selection = command t (Select selection)

let replace t ?if_revision ~selection ~undo text =
  command t (Replace { text; selection; undo; if_revision })
;;

let replace_if_unchanged t expected ~selection ~undo text =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error Input.Command_error.Not_mounted)
  | Some current when not (same_lease current expected) ->
    Bonsai.Effect.return (Error Input.Command_error.Stale_editor)
  | Some _ ->
    send
      t
      expected
      (Replace
         { text; selection; undo; if_revision = Some (Input.Snapshot.revision expected) })
;;
