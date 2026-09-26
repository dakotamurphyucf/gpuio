open Core
open Gpuio
module D = Table_data
module P = Table_paging

let ok = Or_error.ok_exn
let id text = D.Id.of_string text |> ok
let source names = D.create (List.map names ~f:(fun key -> id key, key)) |> ok
let request pager direction = P.request pager direction |> ok |> Option.value_exn
let retry pager direction = P.retry pager direction |> ok |> Option.value_exn
let names pager = D.keys (P.data pager) |> List.map ~f:D.Id.to_string
let show_completion result = print_s [%sexp (result : P.Completion.t)]

let%expect_test "sort reset retires both old pages but preserves surviving keyed anchors" =
  let data = source [ "b"; "c" ] in
  let anchor = D.row_ref data (id "b") |> Option.value_exn in
  let pager =
    P.create ~query:"name ascending" data ~before:(More None) ~after:(More None) |> ok
  in
  let before = request pager Before in
  let after = request pager After in
  assert (Option.is_none (P.request pager Before |> ok));
  let sorted = D.reorder data [ id "c"; id "b" ] |> ok in
  P.reset pager ~query:"name descending" sorted ~before:End ~after:(More None) |> ok;
  assert (D.contains_ref (P.data pager) anchor);
  assert (Option.equal Int.equal (D.index (P.data pager) (D.Row_ref.id anchor)) (Some 1));
  print_s [%sexp (P.Request.query before : string), (P.query pager : string)];
  (* Malformed obsolete responses must not be admitted or mark the new query failed. *)
  show_completion
    (P.complete
       pager
       before
       ~rows:[ id "b", "duplicate" ]
       ~next:(More (Some (String.make 4097 'x')))
     |> ok);
  show_completion (P.fail pager after (Error.of_string "old failure"));
  let current = request pager After in
  show_completion (P.complete pager current ~rows:[ id "a", "a" ] ~next:End |> ok);
  show_completion (P.complete pager current ~rows:[ id "late", "late" ] ~next:End |> ok);
  print_s [%sexp (names pager : string list), (P.generation pager : int64)];
  [%expect
    {|
    ("name ascending" "name descending")
    Obsolete
    Obsolete
    Applied
    Obsolete
    ((c b a) 1)
    |}]
;;

let%expect_test
    "column changes and streamed values do not invalidate in-flight data pages"
  =
  let pager =
    P.create ~query:0 (source [ "middle" ]) ~before:(More (Some "older")) ~after:End |> ok
  in
  let pending = request pager Before in
  let column key =
    Table_column.create ~id:(Table_column.Id.of_string key |> ok) ~label:key () |> ok
  in
  let columns = Table_column.Collection.create [ column "a"; column "b" ] |> ok in
  let columns =
    Table_column.Collection.resize
      columns
      ~column:(Table_column.Id.of_string "a" |> ok)
      ~width:320.
    |> ok
  in
  let columns =
    Table_column.Collection.move
      columns
      ~column:(Table_column.Id.of_string "b" |> ok)
      ~before:(Some (Table_column.Id.of_string "a" |> ok))
    |> ok
  in
  P.set pager ~key:(id "middle") ~data:"streamed" |> ok;
  P.append pager [ id "latest", "latest" ] |> ok;
  show_completion (P.complete pager pending ~rows:[ id "first", "first" ] ~next:End |> ok);
  print_s
    [%sexp
      (names pager : string list), (D.find (P.data pager) (id "middle") : string option)];
  print_s
    [%sexp
      (List.map (Table_column.Collection.to_list columns) ~f:Table_column.label
       : string list)
    , (P.generation pager : int64)];
  [%expect
    {|
    Applied
    ((first middle latest) (streamed))
    ((b a) 0)
    |}]
;;

let%expect_test
    "independent owners, logical cancellation and explicit retry reject late delivery"
  =
  let data = source [] in
  let create () = P.create ~query:() data ~before:End ~after:(More None) |> ok in
  let pager = create () in
  let foreign = create () in
  let old = request pager After in
  ignore (request foreign After : unit P.Request.t);
  show_completion (P.complete foreign old ~rows:[ id "x", "x" ] ~next:End |> ok);
  show_completion (P.cancel pager old);
  let current = request pager After in
  show_completion (P.complete pager old ~rows:[ id "late", "late" ] ~next:End |> ok);
  show_completion (P.fail pager current (Error.of_string "offline"));
  assert (Option.is_none (P.request pager After |> ok));
  let retried = retry pager After in
  show_completion (P.complete pager retried ~rows:[ id "ok", "ok" ] ~next:End |> ok);
  print_s [%sexp (names pager : string list)];
  [%expect
    {|
    Obsolete
    Applied
    Obsolete
    Applied
    Applied
    (ok)
    |}]
;;

let%expect_test
    "bad pages fail atomically; failed reset preserves live requests and query"
  =
  let pager =
    P.create ~query:"original" (source [ "seed" ]) ~before:End ~after:(More None) |> ok
  in
  let old = request pager After in
  assert (
    Result.is_error
      (P.reset
         pager
         ~query:"invalid"
         (source [])
         ~before:End
         ~after:(More (Some (String.make 4097 'x')))));
  assert (String.equal (P.query pager) "original");
  show_completion
    (P.complete pager old ~rows:[ id "next", "next" ] ~next:(More None) |> ok);
  let bad = request pager After in
  assert (
    Result.is_error (P.complete pager bad ~rows:[ id "seed", "duplicate" ] ~next:End));
  assert (Option.is_none (P.request pager After |> ok));
  let bad = retry pager After in
  assert (Result.is_error (P.complete pager bad ~rows:[] ~next:(More None)));
  let bad = retry pager After in
  assert (
    Result.is_error
      (P.complete
         pager
         bad
         ~rows:(List.init 2049 ~f:(fun i -> id (Int.to_string i), "x"))
         ~next:End));
  let bad = retry pager After in
  assert (
    Result.is_error
      (P.complete pager bad ~rows:[] ~next:(More (Some (String.make 4097 'x')))));
  let current = retry pager After in
  show_completion
    (P.complete pager current ~rows:[] ~next:(More (Some (String.make 4096 '\255'))) |> ok);
  let final = request pager After in
  assert (String.length (P.Request.cursor final |> Option.value_exn) = 4096);
  show_completion (P.complete pager final ~rows:[] ~next:End |> ok);
  print_s [%sexp (names pager : string list), (P.generation pager : int64)];
  [%expect
    {|
    Applied
    Applied
    Applied
    ((seed next) 0)
    |}]
;;

let%expect_test "stored failure details are bounded valid text" =
  let pager = P.create ~query:() (source []) ~before:End ~after:(More None) |> ok in
  let pending = request pager After in
  let text = String.concat (List.init 2000 ~f:(fun _ -> "界")) ^ "\000" in
  ignore (P.fail pager pending (Error.of_string text) : P.Completion.t);
  let error =
    match P.status pager After with
    | Failed error -> Error.to_string_hum error
    | Ready | Loading | End -> assert false
  in
  assert (Stdlib.String.is_valid_utf_8 error);
  assert (not (String.contains error '\000'));
  print_s [%sexp (String.length error : int)];
  let pending = retry pager After in
  ignore (P.fail pager pending (Error.of_string "\255") : P.Completion.t);
  print_s [%sexp (P.status pager After : P.Status.t)];
  [%expect
    {|
    4095
    (Failed "Table load failed (invalid UTF-8 error message)")
    |}]
;;

let%expect_test "100k rows load in bounded pages and preserve initial row identity" =
  let data = source [ "000000" ] in
  let anchor = D.row_ref data (id "000000") |> Option.value_exn in
  let pager = P.create ~query:"ascending" data ~before:End ~after:(More None) |> ok in
  let pages = ref 0 in
  let first = ref 1 in
  while !first < 100_000 do
    let count = Int.min P.max_page_rows (100_000 - !first) in
    let rows =
      List.init count ~f:(fun offset ->
        let name = sprintf "%06d" (!first + offset) in
        id name, name)
    in
    first := !first + count;
    let next =
      if !first = 100_000 then P.Boundary.End else More (Some (Int.to_string !first))
    in
    assert (
      P.Completion.equal
        (P.complete pager (request pager After) ~rows ~next |> ok)
        Applied);
    assert (D.contains_ref (P.data pager) anchor);
    incr pages
  done;
  print_s
    [%sexp
      (!pages : int), (D.length (P.data pager) : int), (P.status pager After : P.Status.t)];
  [%expect {| (49 100000 End) |}]
;;
