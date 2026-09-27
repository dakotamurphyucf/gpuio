open Core
open Gpuio
module P = Pagination

let ok = Or_error.ok_exn

let%expect_test "empty, shrink and ordered requests use the latest selection" =
  let t = P.create ~total_pages:10 ~current:8 () |> ok in
  let _, selections =
    List.fold_map
      [ P.Request.next; P.Request.next; P.Request.next; P.Request.previous ]
      ~init:t
      ~f:(fun t request ->
        let t = P.apply_request t request in
        t, P.current t)
  in
  print_s [%sexp (selections : int option list)];
  let small = P.with_total_pages t 3 |> ok in
  let stale = P.apply_request small (P.Request.page 9 |> ok) in
  let empty = P.with_total_pages stale 0 |> ok in
  let grown = P.with_total_pages empty 20 |> ok in
  print_s [%sexp (List.map [ small; stale; empty; grown ] ~f:P.current : int option list)];
  List.iter
    [ P.Request.first
    ; P.Request.previous
    ; P.Request.next
    ; P.Request.last
    ; P.Request.page 2 |> ok
    ]
    ~f:(fun request ->
      let disabled = P.with_disabled t true in
      assert (P.equal disabled (P.apply_request disabled request));
      assert (P.equal empty (P.apply_request empty request)));
  assert (
    Option.equal
      Int.equal
      (P.current (P.select (P.with_disabled t true) ~page:2 |> ok))
      (Some 2));
  assert (Result.is_error (P.select empty ~page:1));
  [%expect
    {|
    ((9) (10) (10) (9))
    ((3) (3) () (1))
    |}]
;;

let%expect_test "bounded page items describe omitted ranges and endpoints" =
  List.iter
    [ P.create ~total_pages:0 () |> ok
    ; P.create ~total_pages:1 () |> ok
    ; P.create ~total_pages:5 ~current:3 ~siblings:0 () |> ok
    ; P.create ~total_pages:100 ~current:50 () |> ok
    ; P.create ~total_pages:P.max_pages ~current:500_000_000 ~siblings:4 () |> ok
    ]
    ~f:(fun t -> print_s [%sexp (P.items t : P.Item.t list)]);
  [%expect
    {|
    ()
    ((Page 1))
    ((Page 1) (Page 2) (Page 3) (Page 4) (Page 5))
    ((Page 1) (Gap (first 2) (last 48)) (Page 49) (Page 50) (Page 51)
     (Gap (first 52) (last 99)) (Page 100))
    ((Page 1) (Gap (first 2) (last 499999995)) (Page 499999996) (Page 499999997)
     (Page 499999998) (Page 499999999) (Page 500000000) (Page 500000001)
     (Page 500000002) (Page 500000003) (Page 500000004)
     (Gap (first 500000005) (last 999999999)) (Page 1000000000))
    |}]
;;

let%expect_test "small exhaustive and maximum-sized page domains have complete partitions"
  =
  let cases = ref 0 in
  let check total current siblings =
    let t = P.create ~total_pages:total ~current ~siblings () |> ok in
    let items = P.items t in
    assert (List.length items <= 13);
    let last =
      List.fold items ~init:0 ~f:(fun previous item ->
        let first, last =
          match item with
          | P.Item.Page page -> page, page
          | Gap { first; last } ->
            assert (last > first);
            assert (current < first || current > last);
            first, last
        in
        assert (first = previous + 1);
        assert (last <= total);
        last)
    in
    assert (last = total);
    assert (List.exists items ~f:(P.Item.equal (Page current)));
    assert (List.exists items ~f:(P.Item.equal (Page 1)));
    assert (List.exists items ~f:(P.Item.equal (Page total)));
    Int.incr cases
  in
  for total = 1 to 64 do
    for current = 1 to total do
      for siblings = 0 to 4 do
        check total current siblings
      done
    done
  done;
  List.iter
    [ 1; 2; 5; 500_000_000; P.max_pages - 4; P.max_pages ]
    ~f:(fun current ->
      for siblings = 0 to 4 do
        check P.max_pages current siblings
      done);
  print_s [%sexp (!cases : int)];
  List.iter
    [ -1; P.max_pages + 1; Int.max_value; Int.min_value ]
    ~f:(fun total_pages ->
      assert (Result.is_error (P.create ~total_pages ()));
      assert (
        Result.is_error
          (P.with_total_pages (P.create ~total_pages:1 () |> ok) total_pages)));
  List.iter
    [ 0; -1; P.max_pages + 1; Int.max_value ]
    ~f:(fun page -> assert (Result.is_error (P.Request.page page)));
  assert (Result.is_error (P.create ~total_pages:0 ~current:1 ()));
  assert (Result.is_error (P.create ~total_pages:2 ~current:3 ()));
  assert (Result.is_error (P.create ~total_pages:2 ~siblings:5 ()));
  assert (Result.is_error (P.create ~total_pages:2 ~siblings:(-1) ()));
  [%expect {| 10430 |}]
;;
