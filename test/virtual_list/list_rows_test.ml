open Core
module C = Gpuio.List_collection
module S = Gpuio.List_selection
module R = Gpuio.List_rows

let ok = Or_error.ok_exn
let source rows = C.of_alist (module Int) rows |> ok

let catalog ?visible ?disabled source =
  S.Catalog.create (C.identity source) ?visible ?disabled () |> ok
;;

let layout ?visible ?disabled ?decorations source =
  R.Layout.create (catalog ?visible ?disabled source) ?decorations () |> ok
;;

let target source key = C.item_ref source key |> Option.value_exn
let row_key rows target = R.item_key rows target |> Option.value_exn

let changed before after =
  C.fold_changed_values
    (R.collection after)
    ~previous:(R.collection before)
    ~init:0
    ~f:(fun n _ -> n + 1)
;;

let%expect_test "source scope identity survives edits and never aliases a fresh source" =
  let a = source [ 1, "one"; 2, "two" ] in
  let id = C.Identity.source_id (C.identity a) in
  List.iter
    [ C.set a ~key:1 ~data:"updated" |> ok
    ; C.reorder a [ 2; 1 ] |> ok
    ; C.splice a ~at:0 ~remove:1 [] |> ok
    ]
    ~f:(fun b -> assert (C.Source_id.equal id (C.Identity.source_id (C.identity b))));
  let b = source [ 1, "one"; 2, "two" ] in
  assert (not (C.Source_id.equal id (C.Identity.source_id (C.identity b))));
  assert (
    not
      (Gpuio.Key.equal
         (C.Source_id.to_key id)
         (C.Source_id.to_key (C.Identity.source_id (C.identity b)))));
  let a = source []
  and b = source [] in
  assert (
    not
      (C.Source_id.equal
         (C.Identity.source_id (C.identity a))
         (C.Identity.source_id (C.identity b))));
  [%expect {| |}]
;;

let%expect_test "decorations do not inflate option positions or counts" =
  let source = source [ 0, "Section"; 1, "Enabled"; 2, "Disabled"; 3, "Footer" ] in
  let layout = layout ~disabled:[ 0; 2; 3 ] ~decorations:[ 0; 3 ] source in
  let rows = R.create source ~layout |> ok in
  assert (R.Layout.option_count layout = 2);
  List.iter
    (C.to_alist (R.collection rows))
    ~f:(fun (_, item) ->
      print_s
        [%sexp
          (R.Item.key item : int)
        , (R.Item.kind item : R.Kind.t)
        , (R.Item.is_disabled item : bool)];
      let a = R.Item.accessibility item ~label:(R.Item.data item) ~selected:true |> ok in
      match R.Item.kind item, a with
      | Decoration, None -> ()
      | Option { index; count }, Some a ->
        (match (Gpuio.Accessibility.Expert.to_wire a).role with
         | Some (Option_item value) ->
           assert (Int.equal value.index index);
           assert (Option.equal Int.equal value.count (Some count));
           assert (value.selected && Bool.equal value.disabled (R.Item.is_disabled item))
         | _ -> assert false)
      | _ -> assert false);
  [%expect
    {|
    (0 Decoration true)
    (1 (Option (index 0) (count 2)) false)
    (2 (Option (index 1) (count 2)) true)
    (3 Decoration true)
    |}]
;;

let%expect_test "bad decoration contracts and stale layout identities are rejected" =
  let a = source [ 0, "Heading"; 1, "Option" ] in
  let c = catalog ~disabled:[ 0 ] a in
  List.iter
    [ [ 0; 0 ]; [ 7 ]; [ 1 ] ]
    ~f:(fun decorations -> assert (Result.is_error (R.Layout.create c ~decorations ())));
  let l = R.Layout.create c ~decorations:[ 0 ] () |> ok in
  let rows = R.create a ~layout:l |> ok in
  let b = C.reorder a [ 1; 0 ] |> ok in
  assert (Result.is_error (R.create b ~layout:l));
  assert (Result.is_error (R.update rows b ~layout:l));
  let b = source [ 0, "Heading"; 1, "Option" ] in
  assert (Result.is_error (R.create b ~layout:l));
  let empty = source [] in
  let l = layout empty in
  assert (R.Layout.option_count l = 0);
  assert (C.is_empty (R.collection (R.create empty ~layout:l |> ok)));
  [%expect {| |}]
;;

let%expect_test
    "filtering retains loaded selections and identities, deletion retires them"
  =
  let a = source [ 1, "one"; 2, "two"; 3, "three" ] in
  let l = layout a in
  let all = R.create a ~layout:l |> ok in
  let one = target a 1
  and two = target a 2 in
  let key_one = row_key all one
  and key_two = row_key all two in
  let selection =
    S.create (R.Layout.catalog l) ~mode:Multiple ~selected:[ 1; 2 ] () |> ok
  in
  let filtered_layout = layout ~visible:[ 1 ] a in
  let filtered = R.update all a ~layout:filtered_layout |> ok in
  let selection = S.reconcile selection (R.Layout.catalog filtered_layout) in
  assert (S.is_selected selection 2);
  assert (C.contains_ref (R.source filtered) two);
  assert (Option.is_none (R.item_key filtered two));
  assert (R.Key.equal key_one (row_key filtered one));
  let restored = R.update filtered a ~layout:l |> ok in
  assert (R.Key.equal key_two (row_key restored two));
  let removed = C.splice a ~at:1 ~remove:1 [] |> ok in
  let inserted = C.splice removed ~at:1 ~remove:0 [ 2, "new two" ] |> ok in
  let l = layout inserted in
  let new_rows = R.update restored inserted ~layout:l |> ok in
  assert (Option.is_none (R.item_key new_rows two));
  assert (not (R.Key.equal key_two (row_key new_rows (target inserted 2))));
  assert (R.Key.equal key_one (row_key new_rows one));
  assert (not (S.is_selected (S.reconcile selection (R.Layout.catalog l)) 2));
  let foreign = source [ 1, "one"; 2, "two"; 3, "three" ] in
  assert (Option.is_none (R.item_key new_rows (target foreign 1)));
  [%expect {| |}]
;;

let%expect_test
    "point updates preserve projected order and only invalidate visible changes"
  =
  let a = source [ 1, ref "one"; 2, ref "hidden"; 3, ref "three" ] in
  let l = layout ~visible:[ 1; 3 ] a in
  let before = R.create a ~layout:l |> ok in
  let source_id = C.Identity.source_id (C.identity (R.collection before)) in
  let a = C.set a ~key:1 ~data:(ref "changed") |> ok in
  let after = R.update before a ~layout:l |> ok in
  assert (changed before after = 1);
  assert (phys_equal (C.keys (R.collection before)) (C.keys (R.collection after)));
  let untouched_key = row_key after (target a 3) in
  assert (
    phys_equal
      (Option.value_exn (R.find before untouched_key))
      (Option.value_exn (R.find after untouched_key)));
  let a = C.set a ~key:2 ~data:(ref "hidden update") |> ok in
  let hidden = R.update after a ~layout:l |> ok in
  assert (phys_equal (R.collection after) (R.collection hidden));
  assert (phys_equal hidden (R.update hidden a ~layout:l |> ok));
  let l = layout a in
  let restored = R.update hidden a ~layout:l |> ok in
  assert (
    C.Source_id.equal
      source_id
      (C.Identity.source_id (C.identity (R.collection restored))));
  let row = R.find restored (row_key restored (target a 2)) |> Option.value_exn in
  assert (String.equal !(R.Item.data row) "hidden update");
  [%expect {| |}]
;;

let%expect_test "all visibility masks retain semantic option order excluding sections" =
  let a = source (List.init 4 ~f:(fun i -> i, i)) in
  for mask = 0 to 15 do
    let visible = List.filter [ 3; 2; 1; 0 ] ~f:(fun i -> mask land (1 lsl i) <> 0) in
    let l = layout ~visible ~disabled:[ 0; 2 ] ~decorations:[ 0 ] a in
    let rows = R.create a ~layout:l |> ok in
    let options = List.filter visible ~f:(fun i -> i <> 0) in
    assert (R.Layout.option_count l = List.length options);
    List.iteri options ~f:(fun index key ->
      let item = R.find rows (row_key rows (target a key)) |> Option.value_exn in
      assert (
        R.Kind.equal (R.Item.kind item) (Option { index; count = List.length options })))
  done;
  [%expect {| |}]
;;

let%expect_test
    "large loaded collections stream without rebuilding the visible projection"
  =
  let a = source (List.init 100_000 ~f:(fun key -> key, key)) in
  let l = layout ~visible:[ 7; 99_999 ] a in
  let before = R.create a ~layout:l |> ok in
  let hidden = C.set a ~key:50_000 ~data:(-1) |> ok in
  let unchanged = R.update before hidden ~layout:l |> ok in
  assert (phys_equal (R.collection before) (R.collection unchanged));
  let updated = C.set hidden ~key:99_999 ~data:(-2) |> ok in
  let after = R.update unchanged updated ~layout:l |> ok in
  assert (changed unchanged after = 1);
  assert (C.length (R.collection after) = 2);
  assert (phys_equal (C.keys (R.collection before)) (C.keys (R.collection after)));
  [%expect {| |}]
;;
