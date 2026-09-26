open Core
open Gpuio
module D = Table_data

let ok = Or_error.ok_exn
let id name = D.Id.of_string name |> ok
let reference data key = D.row_ref data (id key) |> Option.value_exn
let source rows = List.map rows ~f:(fun (key, data) -> id key, data) |> D.create |> ok

let changed data previous =
  D.fold_changed_values data ~previous ~init:[] ~f:(fun keys key ->
    D.Id.to_string key :: keys)
  |> ok
  |> List.sort ~compare:String.compare
;;

let%expect_test
    "membership survives data/order changes but not removal or independent sources"
  =
  let initial = source [ "a", 1; "b", 2 ] in
  let a = reference initial "a" in
  let set = D.set initial ~key:(id "a") ~data:3 |> ok in
  assert (phys_equal (D.keys initial) (D.keys set));
  let reordered = D.reorder set [ id "b"; id "a" ] |> ok in
  let replaced = D.replace reordered [ id "a", 4; id "b", 5 ] |> ok in
  let spliced = D.splice replaced ~at:0 ~remove:1 [ id "a", 6 ] |> ok in
  List.iter [ initial; set; reordered; replaced; spliced ] ~f:(fun data ->
    assert (D.same_source initial data);
    assert (D.contains_ref data a);
    assert (D.Row_ref.equal a (reference data "a")));
  print_s
    [%sexp
      (List.map [ initial; set; reordered; replaced; spliced ] ~f:D.revision : int64 list)];
  print_s
    [%sexp (changed set initial : string list), (changed reordered set : string list)];
  let removed = D.splice spliced ~at:0 ~remove:1 [] |> ok in
  let reintroduced = D.splice removed ~at:0 ~remove:0 [ id "a", 7 ] |> ok in
  assert (not (D.contains_ref removed a));
  assert (not (D.contains_ref reintroduced a));
  assert (not (D.Row_ref.equal a (reference reintroduced "a")));
  let independent = source [ "a", 1; "b", 2 ] in
  assert (not (D.same_source initial independent));
  assert (not (D.contains_ref independent a));
  assert (
    Result.is_error
      (D.fold_changed_values independent ~previous:initial ~init:() ~f:(fun () _ -> ())));
  print_s
    [%sexp
      (D.find initial (id "a") : int option), (D.find reintroduced (id "a") : int option)];
  [%expect
    {|
    (0 1 2 3 4)
    ((a) ())
    ((1) (7))
    |}]
;;

let%expect_test
    "new membership in sibling immutable branches cannot impersonate another branch"
  =
  let empty = source [] in
  let left = D.splice empty ~at:0 ~remove:0 [ id "same", 1 ] |> ok in
  let right = D.splice empty ~at:0 ~remove:0 [ id "same", 2 ] |> ok in
  assert (D.same_source left right);
  assert (Int64.equal (D.revision left) (D.revision right));
  assert (not (D.contains_ref right (reference left "same")));
  assert (not (D.Row_ref.equal (reference left "same") (reference right "same")));
  print_endline
    "revision alone is not identity; membership references distinguish sibling additions";
  [%expect
    {| revision alone is not identity; membership references distinguish sibling additions |}]
;;

let%expect_test "invalid changes are atomic and empty sources have exact range semantics" =
  List.iter
    [ ""; "a\000b"; "\255"; String.make 257 'x' ]
    ~f:(fun text -> assert (Result.is_error (D.Id.of_string text)));
  let initial = source [ "é", 1; "é", 2; "日本語👨‍👩‍👧‍👦", 3 ] in
  assert (D.length initial = 3);
  List.iter
    [ D.replace initial [ id "duplicate", 0; id "duplicate", 1 ]
    ; D.splice initial ~at:1 ~remove:0 [ id "é", 10 ]
    ; D.splice initial ~at:(-1) ~remove:0 []
    ; D.splice initial ~at:4 ~remove:0 []
    ; D.splice initial ~at:1 ~remove:3 []
    ; D.splice initial ~at:1 ~remove:(-1) []
    ; D.reorder initial [ id "é"; id "é"; id "é" ]
    ; D.reorder initial [ id "é" ]
    ; D.set initial ~key:(id "missing") ~data:4
    ]
    ~f:(fun result -> assert (Result.is_error result));
  assert (Int64.equal (D.revision initial) 0L);
  assert (D.length initial = 3);
  let empty = D.replace initial [] |> ok in
  assert (D.is_empty empty);
  assert (Option.is_none (D.nth empty 0));
  assert (Option.is_none (D.nth initial (-1)));
  assert (Option.is_none (D.nth initial 3));
  assert (Result.is_error (D.range initial ~first:2 ~last:1));
  assert (Result.is_error (D.range initial ~first:0 ~last:4));
  print_s [%sexp (D.range empty ~first:0 ~last:0 |> ok : (D.Id.t * int) list)];
  print_s [%sexp (D.range initial ~first:1 ~last:2 |> ok : (D.Id.t * int) list)];
  assert (D.key_bytes empty = 0);
  assert (
    D.key_bytes initial
    = List.sum
        (module Int)
        (D.keys initial)
        ~f:(fun key -> String.length (D.Id.to_string key)));
  [%expect
    {|
    ()
    (("e\204\129" 2))
    |}]
;;

let%expect_test "100k rows retain stable identity through sorting and point updates" =
  let rows = List.init 100_000 ~f:(fun i -> id (sprintf "%06d" i), i) in
  let original = D.create rows |> ok in
  let anchor = reference original "050000" in
  let reordered = D.reorder original (List.rev (D.keys original)) |> ok in
  assert (List.is_empty (changed reordered original));
  assert (D.contains_ref reordered anchor);
  assert (Option.equal Int.equal (D.index reordered (D.Row_ref.id anchor)) (Some 49_999));
  List.iteri (D.keys reordered) ~f:(fun position key ->
    assert (Option.equal Int.equal (D.find reordered key) (Some (99_999 - position)));
    assert (D.contains_ref reordered (D.row_ref original key |> Option.value_exn)));
  let updated =
    List.fold (List.range 0 100) ~init:reordered ~f:(fun data i ->
      let data = D.set data ~key:(id "050000") ~data:i |> ok in
      assert (phys_equal (D.keys reordered) (D.keys data));
      data)
  in
  assert (D.contains_ref updated anchor);
  print_s
    [%sexp
      (D.length updated : int)
    , (D.key_bytes updated : int)
    , (changed updated reordered : string list)];
  print_s
    [%sexp (D.revision updated : int64), (D.find updated (id "050000") : int option)];
  [%expect
    {|
    (100000 600000 (050000))
    (101 (99))
    |}]
;;

let%expect_test "row references keep neither application payloads nor old snapshots alive"
  =
  let weak = Stdlib.Weak.create 2 in
  let[@inline never] make () =
    let payload = Bytes.create 1_048_576 in
    Stdlib.Weak.set weak 0 (Some payload);
    let data = source [ "row", payload ] in
    let row = reference data "row" in
    let data = D.set data ~key:(id "row") ~data:(Bytes.create 32) |> ok in
    let replacement = D.find data (id "row") |> Option.value_exn in
    Stdlib.Weak.set weak 1 (Some replacement);
    row, D.replace data [] |> ok
  in
  let row, empty = make () in
  Gc.full_major ();
  assert (not (Stdlib.Weak.check weak 0));
  assert (not (Stdlib.Weak.check weak 1));
  assert (not (D.contains_ref empty row));
  assert (D.Id.equal (D.Row_ref.id row) (id "row"));
  print_endline
    "both payload versions collected while an obsolete row reference remains live";
  [%expect
    {| both payload versions collected while an obsolete row reference remains live |}]
;;

let%expect_test "logical row and key budgets reject before building an oversized index" =
  let entry = id "same", () in
  let rows = List.init (D.max_rows + 1) ~f:(fun _ -> entry) in
  print_s [%sexp (D.create rows |> Or_error.map ~f:ignore : unit Or_error.t)];
  let entry = id (String.make 256 'x'), () in
  let rows = List.init ((D.max_key_bytes / 256) + 1) ~f:(fun _ -> entry) in
  print_s [%sexp (D.create rows |> Or_error.map ~f:ignore : unit Or_error.t)];
  [%expect
    {|
    (Error "table data exceeds 1000000 rows")
    (Error "table data exceeds 67108864 key bytes")
    |}]
;;
