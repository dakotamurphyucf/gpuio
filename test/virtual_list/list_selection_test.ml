open Core
module C = Gpuio.List_collection
module S = Gpuio.List_selection

let ok = Or_error.ok_exn
let source keys = C.of_alist (module Int) (List.map keys ~f:(fun key -> key, ())) |> ok
let reference source key = C.item_ref source key |> Option.value_exn

let catalog ?visible ?disabled source =
  S.Catalog.create (C.identity source) ?visible ?disabled () |> ok
;;

let key = Option.map ~f:C.Item_ref.key
let selected t = S.selected t |> List.map ~f:C.Item_ref.key

let show t =
  print_s
    [%sexp
      (key (S.cursor t) : int option)
    , (selected t : int list)
    , (key (S.anchor t) : int option)
    , (key (S.context t) : int option)]
;;

let%expect_test "cursor, committed membership and context are independent" =
  let source = source [ 0; 1; 2; 3; 4; 5 ] in
  let catalog = catalog source ~disabled:[ 2; 4 ] in
  let state = S.create catalog () |> ok in
  let state =
    S.navigate state catalog Next
    |> fun state ->
    S.navigate state catalog Next |> fun state -> S.navigate state catalog Next
  in
  show state;
  let state = S.navigate state catalog Last in
  assert (phys_equal state (S.navigate state catalog Next));
  let state = S.navigate state catalog ~boundary:Wrap Next in
  show state;
  let state = S.select state catalog (reference source 3) Toggle in
  let state = S.focus state catalog (reference source 0) in
  let state = S.with_context state catalog (Some (reference source 5)) in
  show state;
  List.iter [ S.Confirmation.Primary; Secondary ] ~f:(fun kind ->
    let target, observed = S.confirm state catalog kind |> Option.value_exn in
    assert (C.Item_ref.key target = 0);
    assert (S.Confirmation.equal kind observed));
  assert (phys_equal state (S.focus state catalog (reference source 2)));
  assert (phys_equal state (S.with_context state catalog (Some (reference source 4))));
  show (S.cancel state catalog);
  [%expect
    {|
    ((3) () () ())
    ((0) () () ())
    ((0) (3) (3) (5))
    (() (3) () ())
  |}]
;;

let%expect_test "range, mode changes and idempotent accessibility selection" =
  let source = source [ 0; 1; 2; 3; 4; 5 ] in
  let catalog = catalog source ~disabled:[ 2; 4 ] in
  let state = S.create catalog ~mode:Multiple ~selected:[ 0; 5 ] () |> ok in
  let state = S.focus state catalog (reference source 1) in
  let state = S.select state catalog (reference source 3) (Range { extend = false }) in
  show state;
  let state = S.select state catalog (reference source 1) Toggle in
  let state = S.navigate state catalog ~selection:(Range { extend = true }) Last in
  show state;
  let state = S.with_mode state catalog Single in
  show state;
  let state = S.with_selected state catalog [ 2 ] |> ok in
  assert (S.is_selected state 2);
  let state = S.set_selected state catalog (reference source 0) true in
  let repeated = S.set_selected state catalog (reference source 0) true in
  assert (List.equal Int.equal (selected state) (selected repeated));
  show repeated;
  show (S.set_selected repeated catalog (reference source 0) false);
  assert (Or_error.is_error (S.with_selected state catalog [ 1; 3 ]));
  assert (Or_error.is_error (S.with_selected state catalog [ 99 ]));
  [%expect
    {|
    ((3) (1 3) (1) ())
    ((5) (1 3 5) (1) ())
    ((5) (5) () ())
    ((5) (0) () ())
    ((5) () () ())
  |}]
;;

let%expect_test
    "query filtering keeps hidden selection and repairs cursor without committing"
  =
  let source = source [ 0; 1; 2; 3; 4; 5 ] in
  let all = catalog source in
  let state = S.create all ~mode:Multiple ~selected:[ 0; 5 ] () |> ok in
  let state = S.select state all (reference source 1) Replace in
  let state = S.with_context state all (Some (reference source 0)) in
  let filtered = catalog source ~visible:[ 3; 5 ] in
  let state = S.reconcile state filtered in
  show state;
  let state = S.select state filtered (reference source 3) (Range { extend = false }) in
  show state;
  let disabled = catalog source ~disabled:[ 0; 1; 2; 3; 4; 5 ] in
  let state = S.reconcile state disabled in
  show state;
  assert (Option.is_none (S.confirm state disabled Primary));
  assert (phys_equal state (S.navigate state disabled ~boundary:Wrap Next));
  let state = S.reconcile state all in
  assert (Option.is_none (S.cursor state));
  assert (List.equal Int.equal (selected state) [ 3; 5 ]);
  [%expect
    {|
    ((5) (1) () ())
    ((3) (3 5) (5) ())
    (() (3 5) () ())
  |}]
;;

let%expect_test "deleted/reused memberships and new sources do not inherit selection" =
  let data = source [ 1; 2; 3 ] in
  let all = catalog data in
  let old = reference data 2 in
  let state = S.create all ~mode:Multiple ~selected:[ 2; 3 ] () |> ok in
  let state = S.focus state all old |> fun state -> S.with_context state all (Some old) in
  let removed = C.splice data ~at:1 ~remove:1 [] |> ok in
  let reused = C.splice removed ~at:1 ~remove:0 [ 2, () ] |> ok in
  let next = catalog reused in
  let state = S.reconcile state next in
  show state;
  assert (not (C.Item_ref.equal Int.equal old (S.cursor state |> Option.value_exn)));
  assert (phys_equal state (S.select state next old Replace));
  assert (phys_equal state (S.set_selected state next old true));
  assert (phys_equal state (S.with_context state next (Some old)));
  let foreign = catalog (source [ 1; 2; 3 ]) in
  show (S.reconcile state foreign);
  assert (S.Mode.equal (S.mode (S.reconcile state foreign)) Multiple);
  [%expect
    {|
    ((2) (3) () ())
    (() () () ())
  |}]
;;

let%expect_test "all eligibility masks agree with simple navigation and range order" =
  let keys = List.init 8 ~f:Fn.id in
  let source = source keys in
  for mask = 0 to 255 do
    let enabled = List.filter keys ~f:(fun key -> mask land (1 lsl key) <> 0) in
    let disabled = List.filter keys ~f:(fun key -> mask land (1 lsl key) = 0) in
    let catalog = catalog source ~disabled in
    let initial = S.create catalog ~mode:Multiple () |> ok in
    let state =
      List.fold enabled ~init:initial ~f:(fun state expected ->
        let next = S.navigate state catalog Next in
        assert (Option.equal Int.equal (key (S.cursor next)) (Some expected));
        next)
    in
    assert (phys_equal state (S.navigate state catalog Next));
    assert (
      Option.equal
        Int.equal
        (key (S.cursor (S.navigate state catalog ~boundary:Wrap Next)))
        (List.hd enabled));
    let reverse =
      List.fold (List.rev enabled) ~init:initial ~f:(fun state expected ->
        let next = S.navigate state catalog Previous in
        assert (Option.equal Int.equal (key (S.cursor next)) (Some expected));
        next)
    in
    assert (phys_equal reverse (S.navigate reverse catalog Previous));
    assert (
      Option.equal
        Int.equal
        (key (S.cursor (S.navigate reverse catalog ~boundary:Wrap Previous)))
        (List.last enabled));
    match List.hd enabled, List.last enabled with
    | Some first, Some last ->
      let ranged =
        S.focus initial catalog (reference source first)
        |> fun state ->
        S.select state catalog (reference source last) (Range { extend = false })
      in
      assert (List.equal Int.equal (selected ranged) enabled)
    | None, None -> assert (Option.is_none (S.confirm state catalog Secondary))
    | None, Some _ | Some _, None -> assert false
  done;
  [%expect {| |}]
;;

let%expect_test
    "catalog validation and large loaded metadata stay independent of row payloads"
  =
  let data = source (List.init 100_000 ~f:Fn.id) in
  let identity = C.identity data in
  let catalog = catalog data ~disabled:[ 1; 2; 3 ] in
  let updated = C.set data ~key:99_999 ~data:() |> ok in
  assert (phys_equal identity (C.identity updated));
  let state =
    S.create catalog ~mode:Multiple ~selected:(List.init 100_000 ~f:Fn.id) () |> ok
  in
  let state =
    List.fold (List.init 1000 ~f:Fn.id) ~init:state ~f:(fun state _ ->
      S.navigate state catalog Next)
  in
  assert (Option.equal Int.equal (key (S.cursor state)) (Some 1002));
  assert (List.length (selected state) = 100_000);
  assert (phys_equal state (S.reconcile state catalog));
  List.iter
    [ [ 1; 1 ]; [ -1 ] ]
    ~f:(fun invalid ->
      assert (Or_error.is_error (S.Catalog.create identity ~visible:invalid ()));
      assert (Or_error.is_error (S.Catalog.create identity ~disabled:invalid ())));
  assert (Or_error.is_error (S.with_selected state catalog [ 1; 1 ]));
  let empty = S.Catalog.create identity ~visible:[] () |> ok in
  let state = S.reconcile state empty in
  assert (Option.is_none (S.cursor state));
  assert (List.length (selected state) = 100_000);
  [%expect {| |}]
;;

let%expect_test
    "mode fallback uses source order and extended ranges preserve hidden choices"
  =
  let data = source [ 9; 3; 1 ] in
  let all = catalog data in
  let state = S.create all ~mode:Multiple ~selected:[ 1; 9 ] () |> ok in
  let state = S.focus state all (reference data 3) in
  assert (List.equal Int.equal (selected (S.with_mode state all Single)) [ 9 ]);
  let filtered = catalog data ~visible:[ 3; 1 ] in
  let state = S.reconcile state filtered in
  let state = S.select state filtered (reference data 1) (Range { extend = true }) in
  assert (List.equal Int.equal (selected state) [ 1; 3; 9 ]);
  assert (phys_equal state (S.set_selected state filtered (reference data 9) false));
  let disabled = catalog data ~disabled:[ 1 ] in
  let state = S.reconcile state disabled in
  assert (phys_equal state (S.set_selected state disabled (reference data 1) false));
  assert (S.is_selected state 1);
  [%expect {| |}]
;;
