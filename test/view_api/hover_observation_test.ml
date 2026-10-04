open Core
open Gpuio
open Gpuio_protocol

let ok = Or_error.ok_exn
let window = Window_id.create ~slot:0L ~generation:1L |> ok

let encode events =
  Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
;;

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let fixture name =
  Eio_main.run (fun env ->
    Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip)
;;

let%expect_test "independent hover frames and invalid event envelopes" =
  let node = Node_id.create ~slot:1L ~generation:2L |> ok in
  let handler = Handler_id.create ~slot:3L ~generation:4L |> ok in
  let event revision = Wire.Event.Hover_changed (window, node, handler, revision, true) in
  let events = encode [ event 5L ] in
  assert (String.equal (hex events) (fixture "hover-event.hex"));
  assert (List.equal Wire.Event.equal (Wire.Event.decode events |> ok) [ event 5L ]);
  for length = 0 to String.length events - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix events length)))
  done;
  assert (Result.is_error (Wire.Event.decode (events ^ "\000")));
  assert (Result.is_error (Wire.Event.decode (String.drop_suffix events 1 ^ "\002")));
  assert (Result.is_error (Wire.Event.decode (encode [ event (-1L) ])));
  let message =
    Wire.Message.Apply
      { window
      ; base = 0L
      ; revision = 1L
      ; operations =
          [ Set_hover_observer (node, Some handler); Set_hover_observer (node, None) ]
      }
  in
  let bytes =
    Bin_prot.Utils.bin_dump Wire.Message.bin_writer_t message |> Bigstring.to_string
  in
  assert (String.equal (hex bytes) (fixture "hover-operation.hex"));
  [%expect {| |}]
;;

let%expect_test
    "hover subscription updates do not retire clicks or use unaccepted callbacks"
  =
  let r = Reconciler.create window in
  let base =
    View.button ~key:(Key.of_string_exn "action") ~on_click:(fun () -> "clicked") "Action"
  in
  let observe label =
    View.with_hover base ~on_change:(fun b -> label ^ Bool.to_string b) |> ok
  in
  let prepare view = Reconciler.prepare r ~theme:Theme.default view |> ok in
  let accept update =
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | _ -> []
  in
  let first = accept (prepare (Some (observe "first:"))) in
  let node, click =
    List.find_map_exn first ~f:(function
      | Wire.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let hover =
    List.find_map_exn first ~f:(function
      | Wire.Op.Set_hover_observer (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (not (Handler_id.equal click hover));
  let event handler = Wire.Event.Hover_changed (window, node, handler, 1L, true) in
  let press = Wire.Event.Press (window, node, click, 1L) in
  let dispatch expected event =
    assert (Option.equal String.equal (Reconciler.dispatch r event) expected)
  in
  dispatch (Some "first:true") (event hover);
  dispatch None (event click);
  let candidate = prepare (Some (observe "latest:")) in
  dispatch (Some "first:true") (event hover);
  assert (List.is_empty (accept candidate));
  dispatch (Some "latest:true") (event hover);
  let removed = accept (prepare (Some base)) in
  assert (
    List.exists removed ~f:(function
      | Wire.Op.Set_hover_observer (_, None) -> true
      | _ -> false));
  dispatch None (event hover);
  dispatch (Some "clicked") press;
  let restored = accept (prepare (Some (observe "restored:"))) in
  assert (
    not
      (List.exists restored ~f:(function
         | Wire.Op.Create _ | Remove _ | Bind _ -> true
         | _ -> false)));
  let next =
    List.find_map_exn restored ~f:(function
      | Wire.Op.Set_hover_observer (_, Some h) -> Some h
      | _ -> None)
  in
  assert (not (Handler_id.equal hover next));
  dispatch None (event hover);
  dispatch
    (Some "restored:true")
    (Wire.Event.Hover_changed (window, node, next, 3L, true));
  dispatch (Some "clicked") press;
  ignore (accept (prepare None) : Wire.Op.t list);
  dispatch None (event next);
  Reconciler.close r;
  dispatch None press;
  print_endline
    "independent handler; accepted callback; old hover retired; click and node retained";
  [%expect
    {| independent handler; accepted callback; old hover retired; click and node retained |}]
;;

let%expect_test "hover roots are explicitly checked" =
  assert (Or_error.is_error (View.with_hover (View.text "Text") ~on_change:ignore));
  assert (Or_error.is_error (View.with_hover (View.column []) ~on_change:ignore));
  [%expect {| |}]
;;
