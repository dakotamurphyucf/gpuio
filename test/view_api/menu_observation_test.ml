open Core
open Gpuio
open Gpuio_protocol

let ok = Or_error.ok_exn

let encode_events events =
  Bin_prot.Utils.bin_dump [%bin_writer: Wire.Event.t list] events |> Bigstring.to_string
;;

let window = Window_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "menu observation bytes and malformed envelopes" =
  let node = Node_id.create ~slot:1L ~generation:2L |> ok in
  let handler = Handler_id.create ~slot:2L ~generation:3L |> ok in
  let event open_ = Wire.Event.Menu_open_changed (window, node, handler, 7L, open_) in
  let bytes = encode_events [ event true; event false ] in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "menu-observation.hex")
         |> String.strip)));
  assert (
    List.equal
      Wire.Event.equal
      (Wire.Event.decode bytes |> ok)
      [ event true; event false ]);
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (Wire.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (Wire.Event.decode (bytes ^ "\000")));
  assert (Result.is_error (Wire.Event.decode (String.drop_suffix bytes 1 ^ "\002")));
  assert (
    Result.is_error
      (Wire.Event.decode
         (encode_events [ Menu_open_changed (window, node, handler, -1L, false) ])));
  [%expect {| |}]
;;

let%expect_test
    "menu observers fence definition changes but retain style and placement updates"
  =
  let r = Reconciler.create window in
  let view ?(label = "Actions") ?(disabled = false) ?placement ?on_open_change () =
    View.menu_button
      ~key:(Key.of_string_exn "menu")
      ?placement
      ?on_open_change
      ~menu:(Menu.create ~label ~disabled [] |> ok)
      ()
  in
  let prepare v = Reconciler.prepare r ~theme:Theme.default v |> ok in
  let accept update =
    Reconciler.accept r update |> ok;
    match Reconciler.message update with
    | Some (Wire.Message.Apply tx) -> tx.operations
    | _ -> []
  in
  let first = accept (prepare (Some (view ~on_open_change:(fun b -> "first", b) ()))) in
  let node, handler =
    List.find_map_exn first ~f:(function
      | Wire.Op.Create (node, Menu, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let event ?(handler = handler) ?(revision = 1L) open_ =
    Wire.Event.Menu_open_changed (window, node, handler, revision, open_)
  in
  assert (
    Option.equal
      [%equal: string * bool]
      (Reconciler.dispatch r (event true))
      (Some ("first", true)));
  let replacement =
    prepare (Some (view ~label:"Replaced" ~on_open_change:(fun b -> "next", b) ()))
  in
  assert (Option.is_some (Reconciler.dispatch r (event true)));
  let operations = accept replacement in
  let next_handler =
    List.find_map_exn operations ~f:(function
      | Wire.Op.Bind (_, Some handler) -> Some handler
      | _ -> None)
  in
  assert (not (Handler_id.equal handler next_handler));
  assert (Option.is_none (Reconciler.dispatch r (event true)));
  let placement = Placement.create ~side:Right ~align:End ~offset:8. () |> ok in
  let operations =
    accept
      (prepare
         (Some
            (view ~label:"Replaced" ~placement ~on_open_change:(fun b -> "latest", b) ())))
  in
  assert (
    List.exists operations ~f:(function
      | Wire.Op.Set_placement (_, Some _) -> true
      | _ -> false));
  assert (
    not
      (List.exists operations ~f:(function
         | Wire.Op.Bind _ -> true
         | _ -> false)));
  assert (
    Option.equal
      [%equal: string * bool]
      (Reconciler.dispatch r (event ~handler:next_handler ~revision:2L false))
      (Some ("latest", false)));
  ignore
    (accept
       (prepare
          (Some
             (view
                ~label:"Replaced"
                ~disabled:true
                ~on_open_change:(fun b -> "disabled", b)
                ())))
     : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch r (event ~handler:next_handler true)));
  ignore (accept (prepare None) : Wire.Op.t list);
  assert (Option.is_none (Reconciler.dispatch r (event ~handler:next_handler false)));
  print_endline
    "unaccepted changes preserve dispatch; replacement/disable/unmount retire old \
     observers; placement keeps latest callback";
  [%expect
    {| unaccepted changes preserve dispatch; replacement/disable/unmount retire old observers; placement keeps latest callback |}]
;;
