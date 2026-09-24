open Core
module C = Gpuio.List_collection
module P = Gpuio.List_paging

let collection rows = C.of_alist (module Int) rows |> Or_error.ok_exn
let rows t = print_s [%sexp (C.to_alist t : (int * string) list)]
let completion result = print_s [%sexp (result : P.Completion.t Or_error.t)]

let%expect_test "streaming replacement preserves order; splices and reorders are atomic" =
  let original = collection [ 2, "two"; 3, "three" ] in
  let updated = C.set original ~key:2 ~data:"streaming" |> Or_error.ok_exn in
  let prepended = C.splice updated ~at:0 ~remove:0 [ 1, "one" ] |> Or_error.ok_exn in
  let reordered = C.reorder prepended [ 3; 1; 2 ] |> Or_error.ok_exn in
  let replaced =
    C.splice reordered ~at:1 ~remove:2 [ 2, "replacement" ] |> Or_error.ok_exn
  in
  List.iter [ original; updated; prepended; reordered; replaced ] ~f:rows;
  print_s
    [%sexp
      (C.index reordered 2 : int option)
    , (C.nth reordered 0 : (int * string) option)
    , (C.range reordered ~first:1 ~last:3 : (int * string) list Or_error.t)];
  let invalid =
    [ C.splice original ~at:0 ~remove:0 [ 3, "duplicate" ]
    ; C.splice original ~at:Int.max_value ~remove:1 []
    ; C.splice original ~at:1 ~remove:Int.max_value []
    ; C.reorder original [ 2; 2 ]
    ; C.reorder original [ 2; 4 ]
    ; C.set original ~key:9 ~data:"missing"
    ]
  in
  print_s [%sexp (List.map invalid ~f:Or_error.is_error : bool list)];
  rows original;
  [%expect
    {|
    ((2 two) (3 three))
    ((2 streaming) (3 three))
    ((1 one) (2 streaming) (3 three))
    ((3 three) (1 one) (2 streaming))
    ((3 three) (2 replacement))
    ((2) ((3 three)) (Ok ((1 one) (2 streaming))))
    (true true true true true true)
    ((2 two) (3 three))
    |}]
;;

let start t direction = P.request t direction |> Or_error.ok_exn |> Option.value_exn

let%expect_test "opposite pages can complete in either order alongside streaming" =
  let t = P.create (collection [ 2, "draft" ]) ~before:(More None) ~after:(More None) in
  let before = start t Before in
  let after = start t After in
  assert (Option.is_none (P.request t Before |> Or_error.ok_exn));
  P.set t ~key:2 ~data:"complete" |> Or_error.ok_exn;
  completion (P.complete t after ~rows:[ 3, "three" ] ~next:End);
  completion (P.complete t before ~rows:[ 0, "zero"; 1, "one" ] ~next:End);
  rows (P.items t);
  print_s [%sexp (P.status t Before : P.Status.t), (P.status t After : P.Status.t)];
  assert (Option.is_none (P.request t After |> Or_error.ok_exn));
  [%expect
    {|
    (Ok Applied)
    (Ok Applied)
    ((0 zero) (1 one) (2 complete) (3 three))
    (End End)
    |}]
;;

let%expect_test "generation, owner, cancellation and retry reject obsolete deliveries" =
  let create () = P.create (collection []) ~before:End ~after:(More None) in
  let t = create () in
  let other = create () in
  let old = start t After in
  let foreign = start other After in
  completion (P.complete t foreign ~rows:[ 10, "foreign" ] ~next:End);
  print_s [%sexp (P.cancel t old : P.Completion.t)];
  let fresh = start t After in
  completion (P.complete t old ~rows:[ 10, "cancelled" ] ~next:End);
  print_s [%sexp (P.fail t fresh (Error.of_string "offline") : P.Completion.t)];
  assert (Option.is_none (P.request t After |> Or_error.ok_exn));
  let retry = P.retry t After |> Or_error.ok_exn |> Option.value_exn in
  completion (P.complete t fresh ~rows:[ 10, "old retry" ] ~next:End);
  P.reset t (collection [ 1, "new conversation" ]) ~before:End ~after:End
  |> Or_error.ok_exn;
  completion (P.complete t retry ~rows:[ 10, "old conversation" ] ~next:End);
  rows (P.items t);
  [%expect
    {|
    (Ok Obsolete)
    Applied
    (Ok Obsolete)
    Applied
    (Ok Obsolete)
    (Ok Obsolete)
    ((1 "new conversation"))
    |}]
;;

let%expect_test "invalid pages fail atomically and require explicit retry" =
  let t = P.create (collection [ 1, "one" ]) ~before:End ~after:(More (Some "cursor")) in
  let request = start t After in
  completion (P.complete t request ~rows:[ 1, "duplicate" ] ~next:End);
  rows (P.items t);
  print_s [%sexp (P.status t After : P.Status.t)];
  let request = P.retry t After |> Or_error.ok_exn |> Option.value_exn in
  completion (P.complete t request ~rows:[] ~next:(More (Some "cursor")));
  let request = P.retry t After |> Or_error.ok_exn |> Option.value_exn in
  completion (P.complete t request ~rows:[] ~next:(More (Some "next")));
  let request = start t After in
  print_s [%sexp (P.Request.cursor request : string option)];
  completion (P.complete t request ~rows:[] ~next:End);
  [%expect
    {|
    (Error "list collection keys must be unique")
    ((1 one))
    (Failed "list collection keys must be unique")
    (Error "an empty list page must advance its cursor or reach end")
    (Ok Applied)
    (next)
    (Ok Applied)
    |}]
;;

let%expect_test "large history point updates and ranges preserve persistent data" =
  let count = 100_000 in
  let t = collection (List.init count ~f:(fun key -> key, Int.to_string key)) in
  let updated =
    List.fold (List.init 100 ~f:Fn.id) ~init:t ~f:(fun t index ->
      C.set t ~key:(index * 997) ~data:"updated" |> Or_error.ok_exn)
  in
  for first = 0 to 999 do
    let first = first * 100 in
    let page = C.range updated ~first ~last:(first + 100) |> Or_error.ok_exn in
    assert (List.length page = 100);
    List.iteri page ~f:(fun offset (key, _) -> assert (key = first + offset))
  done;
  print_s
    [%sexp
      (C.length updated : int)
    , (C.find t 0 : string option)
    , (C.find updated 0 : string option)
    , (C.find updated 99_999 : string option)];
  [%expect {| (100000 (0) (updated) (99999)) |}]
;;

let%expect_test "streaming invalidates the changed row without rebuilding order" =
  let original = collection (List.init 100_000 ~f:(fun key -> key, "data")) in
  let updated = C.set original ~key:50_000 ~data:"streamed" |> Or_error.ok_exn in
  let keys =
    C.fold_changed_keys updated ~previous:original ~init:[] ~f:(fun keys key ->
      key :: keys)
  in
  print_s
    [%sexp (phys_equal (C.keys original) (C.keys updated) : bool), (keys : int list)];
  let deleted = C.splice updated ~at:50_000 ~remove:1 [] |> Or_error.ok_exn in
  assert (Option.is_none (C.find deleted 50_000));
  assert (not (phys_equal (C.keys updated) (C.keys deleted)));
  [%expect {| (true (50000)) |}]
;;
