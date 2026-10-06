open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:1L ~generation:1L |> ok
let observer = Gpuio_protocol.Handler_id.create ~slot:2L ~generation:1L |> ok

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "positioned popup independent bytes and finite domain" =
  let position = Menu.Position.create ~x:120. ~y:80. |> ok in
  let message =
    W.Message.Menu_command
      (7L, window, node, observer, Menu.Expert.command_to_wire (Show position))
  in
  print_endline (hex (W.Message.encode message |> ok));
  print_endline
    (hex
       (Bin_prot.Utils.bin_dump
          [%bin_writer: W.Event.t list]
          [ Menu_result (7L, window, node, observer, Applied) ]
        |> Bigstring.to_string));
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; 1_000_001.; -1_000_001. ]
    ~f:(fun bad ->
      assert (Result.is_error (Menu.Position.create ~x:bad ~y:0.));
      assert (Result.is_error (Menu.Position.create ~x:0. ~y:bad)));
  assert (Result.is_ok (Menu.Position.create ~x:(-1_000_000.) ~y:1_000_000.));
  [%expect
    {|
    1707000101010201000000000000005e400000000000005440
    01510700010101020100 |}]
;;

let%expect_test "context observer captures identity and retires replaced definitions" =
  let r = Reconciler.create window in
  let view label =
    View.context_menu
      ~platform:true
      ~key:(Key.of_string_exn "popup")
      ~menu:(Menu.create ~label [] |> ok)
      ~on_change:Fn.id
      (View.text "Anchor")
  in
  let accept view =
    let update = Reconciler.prepare r ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (W.Message.Apply tx) -> tx
    | _ -> assert false
  in
  let first = accept (view "First") in
  let node, observer =
    List.find_map_exn first.operations ~f:(function
      | W.Op.Create (node, Menu, _, Some observer) -> Some (node, observer)
      | _ -> None)
  in
  let event observer revision =
    W.Event.Menu_open_changed (window, node, observer, revision, false)
  in
  let snapshot =
    Reconciler.dispatch r (event observer first.revision) |> Option.value_exn
  in
  assert (Gpuio_protocol.Node_id.equal (Menu.Expert.node snapshot) node);
  assert (Gpuio_protocol.Handler_id.equal (Menu.Expert.observer snapshot) observer);
  assert (not (Menu.Snapshot.is_open snapshot));
  let second = accept (view "Second") in
  let next =
    List.find_map_exn second.operations ~f:(function
      | W.Op.Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (Option.is_none (Reconciler.dispatch r (event observer first.revision)));
  let replacement =
    Reconciler.dispatch r (event next second.revision) |> Option.value_exn
  in
  assert (not (Menu.Expert.same_owner snapshot replacement));
  print_endline
    "context snapshot has exact identity; definition replacement fences the old \
     subscription";
  [%expect
    {| context snapshot has exact identity; definition replacement fences the old subscription |}]
;;
