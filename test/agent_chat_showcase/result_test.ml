open Core
module F = Gpuio_agent_chat_runtime.Result_data
module Q = F.Query
module D = Gpuio.Table_data
module P = Gpuio.Table_paging

let ok = Or_error.ok_exn
let numbers rows = List.map rows ~f:(fun (_, row) -> F.Row.number row)
let sort column direction = { Gpuio.Table.Sort.column = F.column column; direction }

let request query cursor =
  let pager =
    P.create ~query (D.create [] |> ok) ~before:End ~after:(More cursor) |> ok
  in
  P.request pager After |> ok |> Option.value_exn
;;

let%expect_test "paging preserves complete-query order, filtering and Unicode" =
  let query = Q.create ~sort:(sort "score" Descending) () |> ok in
  let first = F.page (request query None) |> ok in
  let cursor =
    match first.next with
    | More cursor -> cursor
    | End -> assert false
  in
  let second = F.page (request query cursor) |> ok in
  let rows = first.rows @ second.rows in
  print_s
    [%sexp
      (List.length first.rows : int)
    , (List.length second.rows : int)
    , (List.equal Int.equal (numbers rows) (numbers (F.rows query)) : bool)
    , (second.next : P.Boundary.t)];
  print_s
    [%sexp (List.take rows 5 |> List.map ~f:(fun (_, row) -> F.Row.score row) : int list)];
  let filtered = F.rows (Q.with_filter query High_score) in
  print_s
    [%sexp
      (List.for_all filtered ~f:(fun (_, row) -> F.Row.score row >= 80) : bool)
    , (List.length filtered : int)
    , (F.rows (Q.with_filter query Empty) |> List.is_empty : bool)];
  let _, row = List.hd_exn (F.rows (Q.create () |> ok)) in
  print_endline (F.cell row (F.column "summary") |> ok);
  [%expect
    {|
    (24 24 true End)
    (100 97 94 93 90)
    (true 10 true)
    Finding 000001 · 日本語 · 👨‍👩‍👧‍👦
    |}]
;;

let%expect_test "complete reorder preserves membership; removal and return retire it" =
  let query = Q.create () |> ok in
  let initial = D.create (F.rows query) |> ok in
  let target = D.row_ref initial (F.id 1) |> Option.value_exn in
  let reversed =
    F.replace initial (Q.with_sort query (Some (sort "number" Descending)) |> ok) |> ok
  in
  let empty = F.replace reversed (Q.with_filter query Empty) |> ok in
  let returned = F.replace empty query |> ok in
  print_s
    [%sexp
      (D.contains_ref reversed target : bool)
    , (D.index reversed (F.id 1) : int option)
    , (D.contains_ref empty target : bool)
    , (D.contains_ref returned target : bool)
    , (D.same_source initial returned : bool)];
  [%expect {| (true (47) false false true) |}]
;;

let%expect_test "unsupported sorts and malformed/out-of-range cursors reject" =
  let query = Q.create () |> ok in
  print_s
    [%sexp
      (Q.with_sort query (Some (sort "summary" Ascending)) |> Or_error.is_error : bool)];
  print_s
    [%sexp
      (List.map [ "bad"; "-1"; "49"; "100001" ] ~f:(fun cursor ->
         F.page (request query (Some cursor)) |> Or_error.is_error)
       : bool list)];
  [%expect
    {|
    true
    (true true true true)
    |}]
;;

let%expect_test "explicit large fixture has stable deterministic ties" =
  let query = Q.create ~size:Large ~sort:(sort "score" Descending) () |> ok in
  let rows = F.rows query in
  let ordered =
    List.is_sorted rows ~compare:(fun (_, left) (_, right) ->
      let primary = Int.compare (F.Row.score right) (F.Row.score left) in
      if primary = 0
      then Int.compare (F.Row.number left) (F.Row.number right)
      else primary)
  in
  print_s [%sexp (List.length rows : int), (ordered : bool)];
  [%expect {| (100000 true) |}]
;;
