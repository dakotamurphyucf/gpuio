open Core
module Data = Gpuio_agent_chat_runtime.Schedule_data
module C = Gpuio.Calendar

let ok = Or_error.ok_exn
let date text = Date.of_string text

let%expect_test "civil filters support empty, partial, single and bounded ranges" =
  let empty = Data.reviews C.Selection.empty |> ok in
  let single = C.Selection.single (date "2026-10-05") |> ok in
  let range =
    C.Range.create ~first:(date "2026-10-05") ~last:(date "2026-10-09")
    |> ok
    |> C.Selection.range
  in
  print_s [%sexp (List.length empty : int)];
  List.iter [ single; range ] ~f:(fun selection ->
    print_endline (Data.describe selection);
    print_s
      [%sexp (Data.reviews selection |> ok |> List.map ~f:Data.Review.date : Date.t list)]);
  List.iter
    [ C.Selection.range_start (date "2026-10-05") |> ok
    ; C.Selection.single (date "2026-10-04") |> ok
    ; C.Selection.single (date "2026-10-20") |> ok
    ; C.Selection.single (date "2026-11-02") |> ok
    ]
    ~f:(fun selection ->
      print_s [%sexp (Result.is_error (Data.reviews selection) : bool)]);
  let across =
    C.Range.create ~first:(date "2026-10-19") ~last:(date "2026-10-21")
    |> ok
    |> C.Selection.range
  in
  print_s
    [%sexp (Data.reviews across |> ok |> List.map ~f:Data.Review.date : Date.t list)];
  [%expect
    {|
    21
    2026-10-05
    (2026-10-05)
    2026-10-05 – 2026-10-09
    (2026-10-05 2026-10-06 2026-10-07 2026-10-08 2026-10-09)
    true
    true
    true
    true
    (2026-10-19 2026-10-21)
    |}]
;;
