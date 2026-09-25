open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let config
      ?(maximum = 5)
      ?(star_size = 24.)
      ?(disabled = false)
      ?(read_only = false)
      value
  =
  Rating.Config.create ~label:"Quality" ~value ~maximum ~star_size ~disabled ~read_only ()
  |> ok
;;

let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let handler = Gpuio_protocol.Handler_id.create ~slot:0L ~generation:1L |> ok

let%expect_test "rating validates state and reduces bursts against latest model" =
  List.iter [ -1; 6 ] ~f:(fun value ->
    assert (Result.is_error (Rating.Config.create ~label:"Quality" ~value ())));
  List.iter [ 0; 33 ] ~f:(fun maximum ->
    assert (Result.is_error (Rating.Config.create ~label:"Quality" ~value:0 ~maximum ())));
  List.iter [ Float.nan; Float.infinity; 7.9; 128.1 ] ~f:(fun star_size ->
    assert (Result.is_error (Rating.Config.create ~label:"Quality" ~value:0 ~star_size ())));
  List.iter
    [ ""; " "; "\255"; "a\000b"; String.make 4097 'a' ]
    ~f:(fun label -> assert (Result.is_error (Rating.Config.create ~label ~value:0 ())));
  let requests =
    List.init 7 ~f:(fun _ -> Rating.Request.increase)
    @ [ Rating.Request.decrease
      ; Rating.Request.toggle 4 |> ok
      ; Rating.Request.toggle 4 |> ok
      ; Rating.Request.clear
      ; Rating.Request.decrease
      ]
  in
  let _, values =
    List.fold_map requests ~init:(config 0) ~f:(fun model request ->
      let model = Rating.Config.apply_request model request in
      model, Rating.Config.value model)
  in
  print_s [%sexp (values : int list)];
  List.iter
    [ config ~disabled:true 2; config ~read_only:true 2 ]
    ~f:(fun model ->
      List.iter requests ~f:(fun request ->
        assert (Rating.Config.equal model (Rating.Config.apply_request model request))));
  assert (
    Rating.Config.value
      (Rating.Config.apply_request (config ~maximum:1 0) (Rating.Request.set 5 |> ok))
    = 0);
  [%expect {| (1 2 3 4 5 5 5 4 0 4 0 0) |}]
;;

let hex s =
  String.to_list s
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let events_bytes events =
  Bin_prot.Utils.bin_dump [%bin_writer: W.Event.t list] events |> Bigstring.to_string
;;

let requests = [ W.Rating.Request.Set 4; Toggle 3; Increase; Decrease ]

let events =
  List.map requests ~f:(fun request ->
    W.Event.Rating_requested (window, node, handler, 1L, request))
;;

let%expect_test "independent paired rating config and event fixtures" =
  let configs =
    [ config 2
    ; config ~maximum:1 ~star_size:8. ~disabled:true 0
    ; config ~maximum:32 ~star_size:128. ~read_only:true 32
    ]
  in
  let operations =
    List.concat_mapi configs ~f:(fun i config ->
      let node =
        Gpuio_protocol.Node_id.create ~slot:(Int64.of_int i) ~generation:1L |> ok
      in
      [ W.Op.Create (node, Rating, "", None)
      ; Set_rating (node, Rating.Expert.to_wire config)
      ])
  in
  let request =
    W.Message.encode (Apply { window; base = 0L; revision = 1L; operations }) |> ok
  in
  let bytes = events_bytes events in
  Eio_main.run (fun env ->
    List.iter
      [ "rating-request.hex", request; "rating-events.hex", bytes ]
      ~f:(fun (name, bytes) ->
        assert (
          String.equal
            (hex bytes)
            (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / name) |> String.strip))));
  assert (List.equal W.Event.equal events (W.Event.decode bytes |> ok));
  for length = 0 to String.length bytes - 1 do
    assert (Result.is_error (W.Event.decode (String.prefix bytes length)))
  done;
  assert (Result.is_error (W.Event.decode (bytes ^ "\000")));
  List.iter [ W.Rating.Request.Set (-1); Set 33; Toggle 0; Toggle 33 ] ~f:(fun request ->
    assert (
      Result.is_error
        (W.Event.decode
           (events_bytes [ Rating_requested (window, node, handler, 1L, request) ]))));
  print_endline "config bounds, requests, strict decoding and independent bytes agree";
  [%expect {| config bounds, requests, strict decoding and independent bytes agree |}]
;;

let%expect_test "rating bindings retain identity and reject stale or prohibited requests" =
  let reconciler = Reconciler.create window in
  let commit config callback =
    let update =
      Reconciler.prepare
        reconciler
        ~theme:Theme.default
        (Some (View.rating ~config ~on_request:callback ()))
      |> ok
    in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let initial = commit (config 2) (fun _ -> 1) in
  let node, handler =
    List.find_map_exn initial ~f:(function
      | W.Op.Create (node, Rating, "", Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let event ?(revision = 1L) handler request =
    W.Event.Rating_requested (window, node, handler, revision, request)
  in
  assert (
    Option.equal
      Int.equal
      (Reconciler.dispatch reconciler (event handler Increase))
      (Some 1));
  assert (List.is_empty (commit (config 2) (fun _ -> 2)));
  assert (
    Option.equal
      Int.equal
      (Reconciler.dispatch reconciler (event handler Increase))
      (Some 2));
  let ops = commit (config ~maximum:1 0) (fun _ -> 2) in
  assert (
    List.for_all ops ~f:(function
      | W.Op.Set_rating _ -> true
      | _ -> false));
  assert (Option.is_none (Reconciler.dispatch reconciler (event handler (Set 2))));
  let disabled = commit (config ~disabled:true 0) (fun _ -> 2) in
  assert (
    List.exists disabled ~f:(function
      | W.Op.Bind (_, None) -> true
      | _ -> false));
  assert (Option.is_none (Reconciler.dispatch reconciler (event handler Increase)));
  let enabled = commit (config 0) (fun _ -> 2) in
  let next =
    List.find_map_exn enabled ~f:(function
      | W.Op.Bind (_, Some h) -> Some h
      | _ -> None)
  in
  assert (not (Gpuio_protocol.Handler_id.equal next handler));
  assert (Option.is_none (Reconciler.dispatch reconciler (event handler Increase)));
  assert (
    Option.is_none
      (Reconciler.dispatch reconciler (event ~revision:Int64.max_value next Increase)));
  let (_ : W.Op.t list) = commit (config ~read_only:true 0) (fun _ -> 2) in
  assert (Option.is_none (Reconciler.dispatch reconciler (event next Increase)));
  print_endline "stable node, latest callback, current range and handler policy enforced";
  [%expect {| stable node, latest callback, current range and handler policy enforced |}]
;;
