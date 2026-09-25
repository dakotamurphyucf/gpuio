open Core
open Gpuio
module Q = Container_query
module W = Gpuio_protocol.Container_query_wire

let ok = Or_error.ok_exn
let id value = Q.Branch_id.of_string value |> ok
let compact = id "compact"
let wide = id "wide"
let tall = id "tall"
let range ?minimum ?maximum () = Q.Range.create ?minimum ?maximum () |> ok

let rule ?width ?height branch =
  Q.Rule.create ~condition:(Q.Predicate.create ?width ?height ()) ~branch
;;

let config () =
  Q.Config.create
    ~default:compact
    [ rule ~width:(range ~minimum:480.25 ()) ~height:(range ~minimum:200. ()) wide
    ; rule ~height:(range ~minimum:600. ()) tall
    ]
  |> ok
;;

let%expect_test "default, fractional boundaries, height and ordered overlap" =
  let config = config () in
  List.iter
    [ 480.24, 200.; 480.25, 200.; 480.26, 200.; 900., 199.99; 100., 600.; 900., 600. ]
    ~f:(fun (width, height) ->
      Q.Config.select config ~width ~height
      |> ok
      |> Q.Branch_id.to_string
      |> print_endline);
  let half_open =
    Q.Config.create
      ~default:compact
      [ rule ~width:(range ~minimum:480.25 ~maximum:900.5 ()) wide
      ; rule ~height:(range ~minimum:600. ()) tall
      ]
    |> ok
  in
  List.iter [ 900.49; 900.5; 901. ] ~f:(fun width ->
    Q.Config.select half_open ~width ~height:600.
    |> ok
    |> Q.Branch_id.to_string
    |> print_endline);
  let catch_all = Q.Config.create ~default:compact [ rule tall; rule wide ] |> ok in
  Q.Config.select catch_all ~width:900. ~height:600.
  |> ok
  |> Q.Branch_id.to_string
  |> print_endline;
  print_s [%sexp (Q.Config.branches config : Q.Branch_id.t list)];
  [%expect
    {|
    compact
    wide
    wide
    compact
    tall
    wide
    wide
    tall
    tall
    tall
    (compact wide tall)
    |}]
;;

let%expect_test "config matches independently constructed Rust and schema fixture" =
  let config = Q.Expert.to_wire (config ()) ~generation:42L |> ok in
  assert (W.Config.valid config);
  let bytes =
    Bin_prot.Utils.bin_dump [%bin_writer: W.Config.t] config |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let path = Eio.Path.(Eio.Stdenv.fs env / "container-query.hex") in
    assert (String.equal hex (Eio.Path.load path |> String.strip)));
  let native_index = W.Config.select config ~width:480.25 ~height:600. in
  print_s [%sexp (native_index : int64)];
  [%expect {| 1 |}]
;;

let%expect_test "invalid bounds, IDs, dimensions and capacity limits" =
  List.iter [ Float.nan; Float.infinity; -1. ] ~f:(fun value ->
    assert (Result.is_error (Q.Range.create ~minimum:value ()));
    assert (Result.is_error (Q.Range.create ~maximum:value ()));
    assert (Result.is_error (Q.Config.select (config ()) ~width:value ~height:10.));
    assert (Result.is_error (Q.Config.select (config ()) ~width:10. ~height:value)));
  assert (Result.is_error (Q.Range.create ~minimum:2. ~maximum:2. ()));
  assert (Result.is_error (Q.Range.create ~minimum:2. ~maximum:1. ()));
  List.iter
    [ ""; "\000"; "\255"; String.make 129 'a' ]
    ~f:(fun name -> assert (Result.is_error (Q.Branch_id.of_string name)));
  assert (Result.is_ok (Q.Branch_id.of_string (String.make 128 'a')));
  assert (Result.is_ok (Q.Branch_id.of_string "宽"));
  assert (
    Result.is_ok (Q.Config.create ~default:compact (List.init 32 ~f:(fun _ -> rule wide))));
  assert (
    Result.is_error
      (Q.Config.create ~default:compact (List.init 33 ~f:(fun _ -> rule wide))));
  let rules n = List.init n ~f:(fun i -> rule (id (Int.to_string i))) in
  assert (Result.is_ok (Q.Config.create ~default:compact (rules 15)));
  assert (Result.is_error (Q.Config.create ~default:compact (rules 16)));
  List.iter [ 0L; -1L ] ~f:(fun generation ->
    assert (Result.is_error (Q.Expert.to_wire (config ()) ~generation)));
  let empty = Q.Config.create ~default:compact [] |> ok in
  print_s [%sexp (Q.Config.select empty ~width:0. ~height:0. |> ok : Q.Branch_id.t)];
  [%expect {| compact |}]
;;
