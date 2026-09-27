open Core
open Gpuio
module T = Tree
module S = Tree_state
module L = Tree_loading
module R = Tree_rows
module C = List_collection

let ok = Or_error.ok_exn
let id name = T.Id.of_string name |> ok
let ids names = List.map names ~f:id
let leaf data = T.Node.create ~label:"File" ~children:Leaf data |> ok

let branch ?(next = List_paging.Boundary.End) children data =
  T.Node.create ~label:"Folder" ~children:(Branch { ids = ids children; next }) data |> ok
;;

let forest () =
  T.create
    ~roots:(ids [ "root"; "last" ])
    [ id "root", branch ~next:(More None) [ "a"; "nested" ] 0
    ; id "a", leaf 1
    ; id "nested", branch ~next:(More None) [ "b" ] 2
    ; id "b", leaf 3
    ; id "last", leaf 4
    ]
  |> ok
;;

let key rows name = R.item_key rows (id name) |> Option.value_exn
let boundary_key rows name = R.boundary_key rows (id name) |> Option.value_exn

let names rows =
  C.to_alist (R.collection rows)
  |> List.map ~f:(fun (_, row) ->
    match row with
    | R.Row.Item item -> T.Id.to_string item.id
    | Boundary boundary -> "boundary:" ^ T.Id.to_string boundary.parent)
;;

let changed previous rows =
  C.fold_changed_values
    (R.collection rows)
    ~previous:(R.collection previous)
    ~init:[]
    ~f:(fun keys key -> key :: keys)
  |> List.rev
;;

let refresh rows loader state = R.update rows (L.snapshot loader) ~state |> ok

let%expect_test "synthetic boundaries follow subtrees and cannot alias application IDs" =
  let tree = forest () in
  let state = S.create tree ~expanded:(ids [ "root"; "nested" ]) () |> ok in
  let loader = L.create tree in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  print_s [%sexp (names rows : string list)];
  List.iter
    (C.to_alist (R.collection rows))
    ~f:(fun (_, row) ->
      match row with
      | R.Row.Item item ->
        print_s [%sexp (item.id : T.Id.t), (item.position : T.Position.t)]
      | Boundary boundary ->
        assert (boundary.depth = if T.Id.equal boundary.parent (id "root") then 2 else 3));
  let root_key = key rows "root" in
  let child_key = key rows "a" in
  let root_boundary = boundary_key rows "root" in
  assert (not (R.Key.equal root_key root_boundary));
  let state = S.toggle_expanded state tree (id "root") in
  let collapsed = refresh rows loader state in
  print_s [%sexp (names collapsed : string list)];
  assert (Option.is_none (R.find collapsed child_key));
  assert (Option.is_none (R.find collapsed root_boundary));
  assert (R.Key.equal (key collapsed "root") root_key);
  let state = S.toggle_expanded state tree (id "root") in
  let reopened = refresh collapsed loader state in
  assert (not (R.Key.equal (key reopened "a") child_key));
  assert (not (R.Key.equal (boundary_key reopened "root") root_boundary));
  assert (Option.is_none (R.find reopened child_key));
  let long_id = id (String.make 256 'x') in
  let weird_id = id "tree:0:1" in
  let tree =
    T.create ~roots:[ long_id; weird_id ] [ long_id, leaf (); weird_id, leaf () ] |> ok
  in
  let rows =
    R.create (L.snapshot (L.create tree)) ~state:(S.create tree () |> ok) |> ok
  in
  let keys =
    C.keys (R.collection rows)
    |> List.map ~f:(fun key ->
      let wire = R.Key.to_view_key key in
      assert (String.length (Key.to_string wire) <= 256);
      Key.to_string wire)
  in
  assert (Set.length (String.Set.of_list keys) = 2);
  [%expect
    {|
    (root a nested b boundary:nested boundary:root last)
    (root ((parent ()) (depth 1) (index 0) (sibling_count (2))))
    (a ((parent (root)) (depth 2) (index 0) (sibling_count ())))
    (nested ((parent (root)) (depth 2) (index 1) (sibling_count ())))
    (b ((parent (nested)) (depth 3) (index 0) (sibling_count ())))
    (last ((parent ()) (depth 1) (index 1) (sibling_count (2))))
    (root last)
    |}]
;;

let%expect_test
    "coalesced data, selection and cursor updates invalidate only affected rows"
  =
  let tree = forest () in
  let state =
    S.create tree ~mode:Multiple ~expanded:(ids [ "root"; "nested" ]) () |> ok
  in
  let loader = L.create tree in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  let updated_tree = T.set_data tree ~id:(id "a") 100 |> ok in
  let updated_tree = T.set_data updated_tree ~id:(id "b") 200 |> ok in
  L.update loader updated_tree |> ok;
  let updated = refresh rows loader state in
  assert (List.equal R.Key.equal (changed rows updated) [ key rows "a"; key rows "b" ]);
  assert (phys_equal (C.keys (R.collection rows)) (C.keys (R.collection updated)));
  let state = S.select state updated_tree (id "nested") Toggle in
  let selected = refresh updated loader state in
  assert (List.equal R.Key.equal (changed updated selected) [ key rows "nested" ]);
  let state = S.focus state updated_tree (id "last") in
  let focused = refresh selected loader state in
  assert (
    List.equal
      R.Key.equal
      (changed selected focused)
      [ key rows "nested"; key rows "last" ]);
  assert (phys_equal focused (refresh focused loader state));
  let collapsed_state = S.toggle_expanded state updated_tree (id "root") in
  let collapsed = refresh focused loader collapsed_state in
  let hidden_update = T.set_data updated_tree ~id:(id "b") 300 |> ok in
  L.update loader hidden_update |> ok;
  let hidden = refresh collapsed loader collapsed_state in
  assert (phys_equal (R.collection collapsed) (R.collection hidden));
  let reopened =
    refresh hidden loader (S.toggle_expanded collapsed_state hidden_update (id "root"))
  in
  (match R.find reopened (key reopened "b") with
   | Some (Item item) -> assert (T.Node.data item.node = 300)
   | None | Some (Boundary _) -> assert false);
  print_endline
    "two streamed nodes; one selected row; two cursor rows; hidden payload retained";
  [%expect
    {| two streamed nodes; one selected row; two cursor rows; hidden payload retained |}]
;;

let%expect_test
    "load status changes preserve keys and End removes only its boundary identity"
  =
  let tree = forest () in
  let state = S.create tree ~expanded:(ids [ "root"; "nested" ]) () |> ok in
  let loader = L.create tree in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  assert (L.request loader (id "nested") |> ok);
  let queued = refresh rows loader state in
  assert (
    List.equal
      R.Key.equal
      (changed rows queued)
      [ key rows "nested"; boundary_key rows "nested" ]);
  let request = L.take loader |> Option.value_exn in
  let loading = refresh queued loader state in
  assert (List.length (changed queued loading) = 2);
  assert (L.Completion.equal (L.fail loader request (Error.of_string "denied")) Applied);
  let failed = refresh loading loader state in
  assert (List.length (changed loading failed) = 2);
  assert (L.retry loader (id "nested") |> ok);
  let retrying = refresh failed loader state in
  assert (R.Key.equal (boundary_key retrying "nested") (boundary_key rows "nested"));
  let request = L.take loader |> Option.value_exn in
  L.complete loader request { roots = []; nodes = []; next = End }
  |> ok
  |> fun completion ->
  assert (L.Completion.equal completion Applied);
  let completed = refresh retrying loader state in
  assert (Option.is_none (R.boundary_key completed (id "nested")));
  assert (R.Key.equal (key completed "b") (key rows "b"));
  assert (Option.is_none (R.find completed (boundary_key rows "nested")));
  print_s [%sexp (names completed : string list)];
  [%expect {| (root a nested b boundary:root last) |}]
;;

let%expect_test
    "reorder, reincarnation, reset and backwards revisions keep identity explicit"
  =
  let tree = forest () in
  let state = S.create tree ~expanded:(ids [ "root"; "nested" ]) () |> ok in
  let loader = L.create tree in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  let old = L.snapshot loader in
  let foreign = L.snapshot (L.create tree) in
  assert (Int64.equal (L.Snapshot.generation old) (L.Snapshot.generation foreign));
  assert (not (L.Snapshot.same_generation old foreign));
  assert (Result.is_error (R.update rows foreign ~state));
  let tree = T.replace tree ~roots:(ids [ "last"; "root" ]) (T.to_alist tree) |> ok in
  L.update loader tree |> ok;
  let reordered = refresh rows loader state in
  assert (R.Key.equal (key rows "last") (key reordered "last"));
  assert (Result.is_error (R.update reordered old ~state));
  let tree =
    T.replace
      tree
      ~roots:(ids [ "root" ])
      (T.to_alist tree
       |> List.filter ~f:(fun (key, _) -> not (T.Id.equal key (id "last"))))
    |> ok
  in
  (* Deliberately coalesce the deletion and reinsertion before projecting. *)
  let tree =
    T.replace
      tree
      ~roots:(ids [ "last"; "root" ])
      ((id "last", leaf 55) :: T.to_alist tree)
    |> ok
  in
  L.update loader tree |> ok;
  let reused = refresh reordered loader state in
  assert (not (R.Key.equal (key rows "last") (key reused "last")));
  assert (Option.is_none (R.find reused (key rows "last")));
  L.reset loader tree |> ok;
  assert (Result.is_error (R.update reused (L.snapshot loader) ~state));
  let fresh = R.create (L.snapshot loader) ~state:(S.create tree () |> ok) |> ok in
  assert (not (R.Key.equal (key fresh "last") (key reused "last")));
  print_endline
    "reorder preserves; reincarnation retires; reset and rollback are explicit";
  [%expect
    {| reorder preserves; reincarnation retires; reset and rollback are explicit |}]
;;

let%expect_test "error-detail eviction invalidates the still-failed projected row" =
  let roots = List.init 65 ~f:(fun i -> id (Int.to_string i)) in
  let tree =
    T.create ~roots (List.map roots ~f:(fun id -> id, branch ~next:(More None) [] ()))
    |> ok
  in
  let state = S.create tree ~expanded:roots () |> ok in
  let loader = L.create tree in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  let fail id =
    assert (L.request loader id |> ok);
    let request = L.take loader |> Option.value_exn in
    assert (
      L.Completion.equal
        (L.fail loader request (Error.of_string "original detail"))
        Applied)
  in
  List.iter (List.take roots 64) ~f:fail;
  let before = refresh rows loader state in
  fail (List.last_exn roots);
  let after = refresh before loader state in
  let changes = changed before after in
  assert (List.length changes = 4);
  assert (List.mem changes (boundary_key after "0") ~equal:R.Key.equal);
  (match R.find after (boundary_key after "0") with
   | Some (Boundary { status = Failed error; _ }) ->
     print_endline (Error.to_string_hum error)
   | Some (Boundary { status = Ready | Queued | Loading | End; _ }) | Some (Item _) | None
     -> assert false);
  assert (phys_equal (C.keys (R.collection before)) (C.keys (R.collection after)));
  [%expect {| Tree load failed; retry to request this branch again |}]
;;

let%expect_test "100000 selected nodes keep point invalidation and compact stable keys" =
  let roots = List.init 100_000 ~f:(fun i -> id (Int.to_string i)) in
  let tree = T.create ~roots (List.map roots ~f:(fun id -> id, leaf ())) |> ok in
  let state = S.create tree ~mode:Multiple ~selected:roots () |> ok in
  let loader = L.create tree in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  assert (C.length (R.collection rows) = 100_000);
  let changed_tree = T.set_data tree ~id:(id "50000") () |> ok in
  L.update loader changed_tree |> ok;
  let next = refresh rows loader state in
  assert (List.equal R.Key.equal (changed rows next) [ key rows "50000" ]);
  let state = S.focus state changed_tree (id "99999") in
  let focused = refresh next loader state in
  assert (List.equal R.Key.equal (changed next focused) [ key rows "99999" ]);
  assert (phys_equal (C.keys (R.collection rows)) (C.keys (R.collection focused)));
  assert (List.length (S.selected (R.state focused)) = 100_000);
  print_endline "100000 logical rows; one value invalidation; one focus invalidation";
  [%expect {| 100000 logical rows; one value invalidation; one focus invalidation |}]
;;

let%expect_test
    "maximum lazy forest fits two rows per node and has no historical row registry"
  =
  let roots = List.init T.max_nodes ~f:(fun i -> id (Int.to_string i)) in
  let node = branch ~next:(More None) [] () in
  let tree = T.create ~roots (List.map roots ~f:(fun id -> id, node)) |> ok in
  let loader = L.create tree in
  let state = S.create tree ~expanded:roots () |> ok in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  assert (C.length (R.collection rows) = 2 * T.max_nodes);
  let keys = C.keys (R.collection rows) |> List.map ~f:R.Key.to_view_key in
  ignore (Virtual_list.Order.create keys |> ok : Virtual_list.Order.t);
  let rows =
    List.fold (List.range 0 2) ~init:rows ~f:(fun rows _ ->
      let old = boundary_key rows "0" in
      let state = S.toggle_expanded (R.state rows) tree (id "0") in
      let rows = refresh rows loader state in
      assert (C.length (R.collection rows) = (2 * T.max_nodes) - 1);
      assert (Option.is_none (R.find rows old));
      let state = S.toggle_expanded state tree (id "0") in
      let rows = refresh rows loader state in
      assert (not (R.Key.equal old (boundary_key rows "0")));
      rows)
  in
  assert (C.length (R.collection rows) = 2 * T.max_nodes);
  print_endline
    "200000 logical item/boundary rows fit managed order; retired keys never return";
  [%expect
    {| 200000 logical item/boundary rows fit managed order; retired keys never return |}]
;;

let%expect_test "repeated collapse and reopen never reuses retired identities" =
  let tree = forest () in
  let loader = L.create tree in
  let state = S.create tree ~expanded:(ids [ "root"; "nested" ]) () |> ok in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  let _, retired =
    List.fold
      (List.range 0 1000)
      ~init:(rows, Set.empty (module R.Key))
      ~f:(fun (rows, retired) _ ->
        let retired = Set.add retired (key rows "b") in
        let state = S.toggle_expanded (R.state rows) tree (id "root") in
        let collapsed = refresh rows loader state in
        assert (C.length (R.collection collapsed) = 2);
        let state = S.toggle_expanded state tree (id "root") in
        let rows = refresh collapsed loader state in
        assert (not (Set.mem retired (key rows "b")));
        assert (C.length (R.collection rows) = 7);
        rows, retired)
  in
  assert (Set.length retired = 1000);
  print_endline "1000 collapse/reopen cycles; seven current rows; distinct retired keys";
  [%expect {| 1000 collapse/reopen cycles; seven current rows; distinct retired keys |}]
;;

let%expect_test "maximum-depth lazy boundaries unwind in descendant order" =
  let roots = ids [ "0" ] in
  let nodes =
    List.init T.max_depth ~f:(fun depth ->
      let children =
        if depth + 1 < T.max_depth then [ Int.to_string (depth + 1) ] else []
      in
      id (Int.to_string depth), branch ~next:(More None) children ())
  in
  let tree = T.create ~roots nodes |> ok in
  let loader = L.create tree in
  let state = S.create tree ~expanded:(List.map nodes ~f:fst) () |> ok in
  let rows = R.create (L.snapshot loader) ~state |> ok in
  let expected =
    List.map nodes ~f:(fun (id, _) -> T.Id.to_string id)
    @ List.rev_map nodes ~f:(fun (id, _) -> "boundary:" ^ T.Id.to_string id)
  in
  assert (List.equal String.equal (names rows) expected);
  print_endline "128 items followed by 128 boundaries, innermost first";
  [%expect {| 128 items followed by 128 boundaries, innermost first |}]
;;

let%expect_test "latest projection releases replaced payloads without retaining history" =
  let weak = Stdlib.Weak.create 1 in
  let[@inline never] make () =
    let data = Bytes.create 1_048_576 in
    Stdlib.Weak.set weak 0 (Some data);
    let tree = T.create ~roots:(ids [ "payload" ]) [ id "payload", leaf data ] |> ok in
    let loader = L.create tree in
    let state = S.create tree () |> ok in
    let rows = R.create (L.snapshot loader) ~state |> ok in
    L.update loader (T.set_data tree ~id:(id "payload") (Bytes.create 0) |> ok) |> ok;
    refresh rows loader state
  in
  let rows = make () in
  Gc.full_major ();
  assert (not (Stdlib.Weak.check weak 0));
  assert (C.length (R.collection rows) = 1);
  print_endline "replaced 1 MiB payload collected while current projection stays live";
  [%expect {| replaced 1 MiB payload collected while current projection stays live |}]
;;
