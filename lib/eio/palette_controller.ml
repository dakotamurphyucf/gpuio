open Core
module B = Bonsai.Cont
module Palette = Gpuio.Command_palette

type observation =
  | Native of Palette.Snapshot.t
  | Reply of Palette.Snapshot.t * Palette.Snapshot.t
  | Reset

type t =
  { key : Gpuio.Key.t
  ; snapshot : Palette.Snapshot.t option
  ; observe : observation -> unit Bonsai.Effect.t
  ; send :
      Palette.Snapshot.t
      -> if_query_unchanged:bool
      -> Palette.Command.t
      -> (Palette.Snapshot.t, Palette.Command_error.t) Result.t Bonsai.Effect.t
  }

let install previous next =
  match previous with
  | Some old
    when Palette.Expert.same_owner old next
         && Int64.(Palette.Expert.sequence old >= Palette.Expert.sequence next) ->
    previous
  | Some _ | None -> Some next
;;

let apply previous = function
  | Reset -> None
  | Native next -> install previous next
  | Reply (expected, next) ->
    if
      Option.exists previous ~f:(fun current ->
        Palette.Expert.same_owner current expected)
      && Palette.Expert.same_owner expected next
    then install previous next
    else previous
;;

let create window graph =
  let snapshot, observe =
    B.state_machine0
      ~default_model:None
      ~equal:(Option.equal Palette.Snapshot.equal)
      ~apply_action:(fun _ previous action -> apply previous action)
      graph
  in
  let key = B.path_id graph in
  let open B.Let_syntax in
  let%arr snapshot = snapshot
  and observe = observe
  and key = key in
  { key = Gpuio.Key.of_string key |> Or_error.ok_exn
  ; snapshot
  ; observe
  ; send =
      (fun snapshot ~if_query_unchanged command ->
        App.Window.Expert.palette_command window snapshot ~if_query_unchanged command)
  }
;;

let key t = t.key
let snapshot t = t.snapshot
let observe t snapshot = t.observe (Native snapshot)
let reset t = t.observe Reset

let send t expected ~if_query_unchanged command =
  let open Bonsai.Effect.Let_syntax in
  let%bind result = t.send expected ~if_query_unchanged command in
  let%map () =
    match result with
    | Ok next -> t.observe (Reply (expected, next))
    | Error _ -> Bonsai.Effect.Ignore
  in
  result
;;

let command t command =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error Palette.Command_error.Not_mounted)
  | Some expected -> send t expected ~if_query_unchanged:false command
;;

let command_if_query_unchanged t ~expected command =
  match t.snapshot with
  | None -> Bonsai.Effect.return (Error Palette.Command_error.Not_mounted)
  | Some current when not (Palette.Expert.same_owner current expected) ->
    Bonsai.Effect.return (Error Palette.Command_error.Stale_palette)
  | Some _ -> send t expected ~if_query_unchanged:true command
;;

let%expect_test "late replies never regress observations or cross subscription/reset" =
  let ok = Or_error.ok_exn in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok in
  let snapshot generation sequence =
    let observer = Gpuio_protocol.Handler_id.create ~slot:1L ~generation |> ok in
    Palette.Expert.snapshot_of_wire
      ~window
      ~node
      ~observer
      { sequence
      ; query_revision = sequence
      ; query = Int64.to_string sequence
      ; composing = false
      ; selected = None
      ; matched_count = 0
      ; loading = false
      }
    |> ok
  in
  let old = snapshot 1L 1L
  and newer = snapshot 1L 3L
  and replacement = snapshot 2L 1L in
  let state = apply None (Native old) in
  let state = apply state (Native newer) in
  assert (
    Option.equal Palette.Snapshot.equal (apply state (Reply (old, snapshot 1L 2L))) state);
  let state = apply state (Native replacement) in
  assert (
    Option.equal Palette.Snapshot.equal (apply state (Reply (old, snapshot 1L 4L))) state);
  let state = apply state Reset in
  assert (Option.is_none (apply state (Reply (replacement, snapshot 2L 2L))));
  print_endline
    "newer observation, replacement subscription and reset survive late replies";
  [%expect
    {| newer observation, replacement subscription and reset survive late replies |}]
;;
