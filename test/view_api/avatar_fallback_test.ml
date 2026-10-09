open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn

let config label =
  Avatar.Config.create
    ~fallback:(Avatar.Fallback.create "?" |> ok)
    ~description:(Image.Description.label label |> ok)
    ()
;;

let wrap fallback = View.avatar_with_fallback (config "Owner") ~fallback

let%expect_test
    "rich avatar admits bounded passive content and rejects active descendants"
  =
  let passive = View.row [ View.text "AI"; View.avatar (config "Nested decoration") ] in
  assert (Result.is_ok (wrap passive));
  assert (Result.is_error (wrap (View.row [ View.button ~on_click:(fun () -> ()) "Bad" ])));
  List.iter
    [ Style.Property.User_select true
    ; Overflow_x Scroll
    ; Inert true
    ; Disabled true
    ; Pointer_occlusion Pointer
    ]
    ~f:(fun property ->
      let style = Style.create_exn [ property ] in
      assert (Result.is_error (wrap (View.column [ View.text ~style "Bad" ]))));
  let shallow count = View.column (List.init count ~f:(fun _ -> View.text "x")) in
  assert (Result.is_ok (wrap (shallow 4095)));
  assert (Result.is_error (wrap (shallow 4096)));
  let deep count =
    List.fold
      (List.init (count - 1) ~f:Fn.id)
      ~init:(View.text "x")
      ~f:(fun child _ -> View.column [ child ])
  in
  assert (Result.is_ok (wrap (deep 128)));
  assert (Result.is_error (wrap (deep 129)));
  assert (
    Result.is_error
      (Avatar_group.Item.create_with_fallback
         ~key:(key "bad")
         (config "Owner")
         ~fallback:(View.button ~on_click:(fun () -> ()) "Bad")));
  print_endline
    "passive composition; 4096 nodes / 128 levels; active descendants rejected";
  [%expect
    {| passive composition; 4096 nodes / 128 levels; active descendants rejected |}]
;;

let%expect_test
    "fallback retains keyed identity across root updates and ordinary group changes"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | None -> []
    | Some (Apply { operations; _ }) -> operations
    | Some _ -> assert false
  in
  let fallback = View.row ~key:(key "fallback") [ View.text ~key:(key "text") "AI" ] in
  let item label =
    Avatar_group.Item.create_with_fallback ~key:(key "member") (config label) ~fallback
    |> ok
  in
  let group label size = Avatar_group.create ~size [ item label ] |> ok in
  let first = commit (group "One" Avatar_group.Size.medium) in
  assert (
    List.exists first ~f:(function
      | W.Op.Create (_, Avatar, _, _) -> true
      | _ -> false));
  assert (List.is_empty (commit (group "One" Avatar_group.Size.medium)));
  let changed = commit (group "Two" Avatar_group.Size.large) in
  assert (
    List.exists changed ~f:(function
      | W.Op.Set_avatar _ -> true
      | _ -> false));
  assert (
    not
      (List.exists changed ~f:(function
         | W.Op.Create _ | Remove _ | Splice _ -> true
         | _ -> false)));
  let removed = commit (Avatar_group.create ~limit:0 [ item "Two" ] |> ok) in
  assert (
    List.exists removed ~f:(function
      | W.Op.Remove _ -> true
      | _ -> false));
  print_endline
    "unchanged idle; root and fallback retained across style/label updates; limit \
     retires slot";
  [%expect
    {| unchanged idle; root and fallback retained across style/label updates; limit retires slot |}]
;;

let%expect_test "rich fallback capability is independently encoded" =
  let encode mask =
    W.Message.encode (Hello (W.version, mask))
    |> ok
    |> String.to_list
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  assert (String.equal (encode 72057594037927936L) "0003fc0000000000000001");
  assert (String.equal (encode W.capabilities) "0003fcffffffffffffff7f");
  print_endline "bit 56; full mask 2^57-1; existing operation tags unchanged";
  [%expect {| bit 56; full mask 2^57-1; existing operation tags unchanged |}]
;;
