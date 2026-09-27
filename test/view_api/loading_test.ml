open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn

let config ?(animated = true) ?(period = 1200.) kind =
  Loading.Config.create
    ~kind
    ~label:"Load"
    ~animated
    ~period:(Time_ns.Span.of_ms period)
    ()
  |> ok
;;

let%expect_test "loading validates periods and labels and matches the Rust fixture" =
  List.iter
    [ ""; " \t"; "\255"; "a\000b"; String.make 4097 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (Loading.Config.create ~kind:Spinner ~label ())));
  List.iter [ 0.; 99.; 60001. ] ~f:(fun ms ->
    assert (
      Result.is_error
        (Loading.Config.create
           ~kind:Spinner
           ~label:"Load"
           ~period:(Time_ns.Span.of_ms ms)
           ())));
  assert ((Loading.Expert.to_wire (config ~period:100.5 Spinner)).period_ms = 101);
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let operations =
    List.concat_mapi
      [ Loading.Kind.Skeleton, 100.; Shimmer, 1200.; Spinner, 60000. ]
      ~f:(fun i (kind, period) ->
        let node =
          Gpuio_protocol.Node_id.create ~slot:(Int64.of_int i) ~generation:1L |> ok
        in
        [ W.Op.Create (node, Loading, "", None)
        ; Set_loading (node, Loading.Expert.to_wire (config ~period kind))
        ])
  in
  let bytes =
    W.Message.encode (Apply { window; base = 0L; revision = 1L; operations }) |> ok
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "loading-request.hex") |> String.strip
    in
    assert (String.equal expected hex));
  print_endline
    "three loading kinds, native period bounds, ceil milliseconds, strict labels, \
     matching fixture";
  [%expect
    {| three loading kinds, native period bounds, ceil milliseconds, strict labels, matching fixture |}]
;;

let%expect_test "loading updates are callback-free and preserve leaf identity" =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let t = Reconciler.create window in
  let commit config =
    let view = View.loading ~config () in
    let update = Reconciler.prepare t ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept t update |> ok;
    Reconciler.message update
  in
  let initial = commit (config Skeleton) in
  let node =
    match initial with
    | Some (Apply { operations; _ }) ->
      List.find_map_exn operations ~f:(function
        | Create (id, Loading, "", None) -> Some id
        | _ -> None)
    | _ -> assert false
  in
  assert (Option.is_none (commit (config Skeleton)));
  List.iter
    [ config Shimmer; config Spinner; config ~animated:false Spinner ]
    ~f:(fun config ->
      match commit config with
      | Some (Apply { operations = [ Set_loading (id, _) ]; _ }) ->
        assert (Gpuio_protocol.Node_id.equal id node)
      | _ -> assert false);
  print_endline
    "same native leaf; no handlers or repeated transactions; static update retains \
     identity";
  [%expect
    {| same native leaf; no handlers or repeated transactions; static update retains identity |}]
;;
