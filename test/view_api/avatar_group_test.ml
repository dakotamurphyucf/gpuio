open Core
open Gpuio
module A = Avatar_group
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let key = Key.of_string_exn

let config ?asset name =
  Avatar.Config.create
    ?asset
    ~fallback:(Avatar.Fallback.create name |> ok)
    ~description:(Image.Description.label name |> ok)
    ()
;;

let item name = A.Item.create ~key:(key name) (config name)

let rec descriptions view =
  let d = View.Expert.describe view in
  d :: List.concat_map d.children ~f:descriptions
;;

let avatars view = List.filter (descriptions view) ~f:(fun d -> Option.is_some d.avatar)

let%expect_test
    "avatar group validates visible and omitted identities and explicit geometry policy"
  =
  let items = [ item "a"; item "b"; item "c"; item "d" ] in
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -0.1; 1.; 1.1 ]
    ~f:(fun overlap -> assert (Result.is_error (A.create ~overlap items)));
  assert (Result.is_error (A.create ~limit:(-1) items));
  assert (Result.is_error (A.create ~limit:0 [ item "a"; item "a" ]));
  List.iter [ Float.nan; Float.infinity; 0.; -1.; 1_000_001. ] ~f:(fun value ->
    assert (Result.is_error (A.Size.of_pixels value)));
  List.iter [ Float.min_positive_subnormal_value; 0.5; 80.; 1_000_000. ] ~f:(fun value ->
    let size = A.Size.of_pixels value |> ok in
    assert (Float.equal (A.Size.pixels size) value);
    ignore (A.create ~size items |> ok));
  let calls = ref [] in
  let overflow ~omitted ~size =
    calls := (omitted, A.Size.pixels size) :: !calls;
    A.ellipsis ~size ~description:(Image.Description.label "More members" |> ok) ()
  in
  let empty = A.create ~overflow [] |> ok in
  assert (List.is_empty (avatars empty));
  let exact = A.create ~overflow ~limit:4 items |> ok in
  assert (List.length (avatars exact) = 4);
  assert (List.is_empty !calls);
  let zero = A.create ~size:A.Size.small ~overflow ~limit:0 items |> ok in
  assert (List.length (avatars zero) = 1);
  assert ([%equal: (int * float) list] !calls [ 4, 24. ]);
  let limited = A.create ~overflow ~limit:2 items |> ok in
  assert (List.map (avatars limited) ~f:(fun d -> d.text) |> List.length = 3);
  assert ([%equal: (int * float) list] !calls [ 2, 48.; 4, 24. ]);
  let no_overflow = A.create ~limit:0 items |> ok in
  assert (List.is_empty (View.Expert.describe no_overflow).children);
  let all = A.create ~limit:Int.max_value items |> ok in
  assert (List.length (avatars all) = 4);
  print_endline
    "bounded overlap and size; duplicates rejected even at limit zero; exact omitted \
     count; no unnecessary overflow construction";
  [%expect
    {| bounded overlap and size; duplicates rejected even at limit zero; exact omitted count; no unnecessary overflow construction |}]
;;

let%expect_test
    "shared dimensions and overlap reach native avatar leaves in logical order"
  =
  let custom =
    Style.create_exn
      [ Width (Length.px_exn 999.)
      ; Shrink 1.
      ; Margin_left (Length.px_exn 99.)
      ; Font_size 17.
      ; Radius 6.
      ]
  in
  let view =
    A.create
      ~size:A.Size.small
      ~limit:3
      ~overlap:0.25
      [ A.Item.create ~key:(key "overflow") ~style:custom (config "a")
      ; A.Item.create ~key:(key "b") ~style:custom (config "b")
      ; A.Item.create ~key:(key "c") ~style:custom (config "c")
      ]
    |> ok
  in
  List.iteri (avatars view) ~f:(fun index d ->
    let fields =
      match Style.Expert.to_wire d.style ~theme:Theme.default |> ok with
      | [ W.Style.Fields fields ] -> fields
      | styles -> raise_s [%message "unexpected style" (styles : W.Style.t list)]
    in
    List.iter
      [ W.Field.Width (Px 24.)
      ; Height (Px 24.)
      ; Min_width (Px 24.)
      ; Max_width (Px 24.)
      ; Shrink 0.
      ; Grow 0.
      ; Font_size 17.
      ; Top_left_radius 6.
      ; Margin_left (Px (if index = 0 then 0. else -6.))
      ]
      ~f:(fun field -> assert (List.mem fields field ~equal:W.Field.equal)));
  assert (
    List.equal
      (Option.equal Key.equal)
      (List.map (avatars view) ~f:(fun d -> d.key))
      (List.map [ "overflow"; "b"; "c" ] ~f:(fun name -> Some (key name))));
  print_endline
    "native leaves get size and overlap; custom font/radius preserved; input order and \
     user keys preserved";
  [%expect
    {| native leaves get size and overlap; custom font/radius preserved; input order and user keys preserved |}]
;;

let%expect_test
    "avatar group limit and reorder preserve surviving identities and fence retired \
     observers"
  =
  let owner = Asset.Expert.Owner.create () in
  let asset =
    Asset.Expert.handle
      ~owner
      ~format:Png
      ~id:(Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok)
  in
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create ~asset_owner:owner window in
  let items names revision =
    List.map names ~f:(fun name ->
      A.Item.create
        ~key:(key name)
        ~on_change:(fun _ -> revision ^ name)
        (config ~asset name))
  in
  let commit ?(size = A.Size.medium) ~limit names revision =
    let view = A.create ~size ~limit (items names revision) |> ok in
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let events operations =
    List.filter_map operations ~f:(function
      | W.Op.Create (node, Avatar, _, Some handler) ->
        Some (node, W.Event.Image_state (window, node, handler, 1L, W.Image.State.Loading))
      | _ -> None)
  in
  let initial = commit ~limit:2 [ "a"; "b"; "c" ] "first-" in
  let first = events initial in
  assert (List.length first = 2);
  let a, ae = List.nth_exn first 0
  and b, be = List.nth_exn first 1 in
  assert (Option.equal String.equal (Reconciler.dispatch reconciler ae) (Some "first-a"));
  let changed = commit ~size:A.Size.large ~limit:2 [ "b"; "a"; "c" ] "latest-" in
  assert (List.is_empty (events changed));
  List.iter changed ~f:(function
    | W.Op.Remove node ->
      assert (
        not (Gpuio_protocol.Node_id.equal node a || Gpuio_protocol.Node_id.equal node b))
    | _ -> ());
  assert (Option.equal String.equal (Reconciler.dispatch reconciler ae) (Some "latest-a"));
  assert (Option.equal String.equal (Reconciler.dispatch reconciler be) (Some "latest-b"));
  assert (List.is_empty (commit ~size:A.Size.large ~limit:2 [ "b"; "a"; "c" ] "latest-"));
  ignore (commit ~limit:1 [ "b"; "a"; "c" ] "kept-" : W.Op.t list);
  assert (Option.is_none (Reconciler.dispatch reconciler ae));
  assert (Option.equal String.equal (Reconciler.dispatch reconciler be) (Some "kept-b"));
  let remounted = commit ~limit:3 [ "a"; "b"; "c" ] "restored-" |> events in
  assert (List.length remounted = 2);
  assert (Option.is_none (Reconciler.dispatch reconciler ae));
  assert (
    Option.equal String.equal (Reconciler.dispatch reconciler be) (Some "restored-b"));
  ignore (commit ~limit:0 [ "a"; "b"; "c" ] "none-" : W.Op.t list);
  List.iter (first @ remounted) ~f:(fun (_, event) ->
    assert (Option.is_none (Reconciler.dispatch reconciler event)));
  Reconciler.close reconciler;
  print_endline
    "reorder/size keep observers and latest closures; limiting unmounts omitted avatars; \
     remounts use fresh identities; zero limit fences all";
  [%expect
    {| reorder/size keep observers and latest closures; limiting unmounts omitted avatars; remounts use fresh identities; zero limit fences all |}]
;;

let%expect_test "overflow updates retain current action and removal fences queued presses"
  =
  let window = Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> ok in
  let reconciler = Reconciler.create window in
  let build ?overflow limit =
    A.create ~limit ?overflow [ item "a"; item "b"; item "c" ] |> ok
  in
  let action ~omitted ~size:_ =
    View.button ~on_click:(fun () -> omitted) "Show remaining"
  in
  let commit view =
    let update = Reconciler.prepare reconciler ~theme:Theme.default (Some view) |> ok in
    Reconciler.accept reconciler update |> ok;
    match Reconciler.message update with
    | Some (Apply tx) -> tx.operations
    | None -> []
    | Some _ -> assert false
  in
  let button operations =
    List.find_map_exn operations ~f:(function
      | W.Op.Create (node, Button, _, Some handler) -> Some (node, handler)
      | _ -> None)
  in
  let node, handler = commit (build ~overflow:action 1) |> button in
  let event = W.Event.Press (window, node, handler, 1L) in
  assert (Option.equal Int.equal (Reconciler.dispatch reconciler event) (Some 2));
  let changes = commit (build ~overflow:action 2) in
  List.iter changes ~f:(function
    | W.Op.Create (_, Button, _, _) -> failwith "overflow action replaced"
    | Remove removed when Gpuio_protocol.Node_id.equal removed node ->
      failwith "overflow action removed"
    | _ -> ());
  assert (Option.equal Int.equal (Reconciler.dispatch reconciler event) (Some 1));
  (* The overflow remains when the entire member container disappears. *)
  ignore (commit (build ~overflow:action 0) : W.Op.t list);
  assert (Option.equal Int.equal (Reconciler.dispatch reconciler event) (Some 3));
  ignore (commit (build 0) : W.Op.t list);
  assert (Option.is_none (Reconciler.dispatch reconciler event));
  let replacement, _ = commit (build ~overflow:action 0) |> button in
  assert (not (Gpuio_protocol.Node_id.equal node replacement));
  assert (Option.is_none (Reconciler.dispatch reconciler event));
  Reconciler.close reconciler;
  print_endline
    "overflow keeps identity and latest omitted count; empty member container does not \
     reparent it; removed actions remain stale after remount";
  [%expect
    {| overflow keeps identity and latest omitted count; empty member container does not reparent it; removed actions remain stale after remount |}]
;;
