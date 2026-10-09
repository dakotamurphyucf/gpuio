open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok
let node = Gpuio_protocol.Node_id.create ~slot:0L ~generation:1L |> ok
let config = Rating.Config.create ~label:"Quality" ~value:2 () |> ok

let hex s =
  String.to_list s
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let%expect_test "independent appearance bytes and capability" =
  let appearances : W.Rating.Appearance.t option list =
    [ Some { active = Some 0x11223344L; inactive = Some 0xffffffffL }
    ; Some { active = None; inactive = Some 0L }
    ; Some { active = None; inactive = None }
    ; None
    ]
  in
  let operations =
    List.map appearances ~f:(fun a -> W.Op.Set_rating_appearance (node, a))
  in
  let bytes =
    W.Message.encode (Apply { window; base = 0L; revision = 1L; operations }) |> ok
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "rating-appearance.hex") |> String.strip
    in
    assert (String.equal (hex bytes) expected));
  let hello mask = W.Message.encode (Hello (W.version, mask)) |> ok |> hex in
  assert (String.equal (hello 144115188075855872L) "0003fc0000000000000002");
  assert (String.equal (hello W.capabilities) "0003fcffffffffffffff7f");
  print_endline
    "operation 64: independent colors, transparent override, default and reset; \
     capability 57";
  [%expect
    {| operation 64: independent colors, transparent override, default and reset; capability 57 |}]
;;

let%expect_test "theme updates and resets retain rating identity and callbacks" =
  let reconciler = Reconciler.create window in
  let view ?appearance callback =
    View.rating ?appearance ~config ~on_request:(fun _ -> callback) ()
  in
  let commit ~theme view =
    let update = Reconciler.prepare reconciler ~theme (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply { operations; _ }) -> operations
    | None -> []
    | Some _ -> assert false
  in
  let theme color = Theme.create [ "active", Color.rgb_exn color ] |> ok in
  let appearance =
    Rating.Appearance.create
      ~active:(Color.token_exn "active")
      ~inactive:(Color.rgb_exn 0x445566 |> fun color -> Color.with_opacity color 0.5 |> ok)
      ()
  in
  let first = commit ~theme:(theme 0x112233) (view ~appearance 1) in
  let node, handler =
    List.find_map_exn first ~f:(function
      | W.Op.Create (node, Rating, "", Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let expect_appearance operations active =
    match operations with
    | [ W.Op.Set_rating_appearance (same, Some colors) ] ->
      assert (Gpuio_protocol.Node_id.equal same node);
      assert (Option.equal Int64.equal colors.active (Some active));
      assert (Option.equal Int64.equal colors.inactive (Some 0x44556680L))
    | _ -> assert false
  in
  expect_appearance
    (List.filter first ~f:(function
       | W.Op.Set_rating_appearance _ -> true
       | _ -> false))
    0x112233ffL;
  expect_appearance (commit ~theme:(theme 0xaabbcc) (view ~appearance 2)) 0xaabbccffL;
  assert (List.is_empty (commit ~theme:(theme 0xaabbcc) (view ~appearance 3)));
  let event = W.Event.Rating_requested (window, node, handler, 1L, Increase) in
  assert (Option.equal Int.equal (Reconciler.dispatch reconciler event) (Some 3));
  let revision = Reconciler.revision reconciler in
  assert (
    Result.is_error
      (Reconciler.prepare reconciler ~theme:Theme.default (Some (view ~appearance 4))));
  assert (Int64.equal revision (Reconciler.revision reconciler));
  assert (Option.equal Int.equal (Reconciler.dispatch reconciler event) (Some 3));
  (match commit ~theme:Theme.default (view 5) with
   | [ W.Op.Set_rating_appearance (same, None) ] ->
     assert (Gpuio_protocol.Node_id.equal same node)
   | _ -> assert false);
  assert (
    List.is_empty
      (commit ~theme:Theme.default (view ~appearance:Rating.Appearance.default 6)));
  assert (Option.equal Int.equal (Reconciler.dispatch reconciler event) (Some 6));
  print_endline
    "resolved theme/alpha; appearance-only changes; missing token is atomic; defaults \
     normalize; latest callback";
  [%expect
    {| resolved theme/alpha; appearance-only changes; missing token is atomic; defaults normalize; latest callback |}]
;;
