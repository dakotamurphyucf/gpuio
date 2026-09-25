open Core
open Gpuio
module N = Numeric

let ok = Or_error.ok_exn
let domain min max step = N.Domain.create ~min ~max ~step |> ok
let show value = printf "%.12g " value

let%expect_test "min anchored grid, irregular endpoint, ties and ordered stepping" =
  let d = domain 0.1 1. 0.2 in
  List.iter [ -1.; 0.1; 0.29; 0.31; 0.51; 0.89; 0.99; 2. ] ~f:(fun value ->
    N.Domain.normalize d value |> ok |> show);
  print_endline "";
  let walk start direction =
    ignore
      (List.fold (List.init 7 ~f:Fn.id) ~init:start ~f:(fun value _ ->
         let next = N.Domain.advance d value ~direction |> ok in
         show next;
         next)
       : float);
    print_endline ""
  in
  walk 0.1 Increase;
  walk 1. Decrease;
  let exact = domain (-1.) 1. 0.5 in
  List.iter [ -0.75; -0.25; 0.25; 0.75 ] ~f:(fun v ->
    N.Domain.normalize exact v |> ok |> show);
  print_endline "";
  [%expect
    {|
    0.1 0.1 0.3 0.3 0.5 0.9 1 1
    0.3 0.5 0.7 0.9 1 1 1
    0.9 0.7 0.5 0.3 0.1 0.1 0.1
    -0.5 0 0.5 1
  |}]
;;

let%expect_test "domain guards, finite input and fixed interval" =
  List.iter
    [ Float.nan, 1., 1.
    ; 0., Float.infinity, 1.
    ; 0., 1., Float.nan
    ; 1., 0., 1.
    ; 0., 1., 0.
    ; 0., 1., -1.
    ; 0., 1., 1e-13
    ; 1e16, 1e16 +. 10., 1.
    ; -1e308, 1e308, 1e300
    ]
    ~f:(fun (min, max, step) ->
      assert (Result.is_error (N.Domain.create ~min ~max ~step)));
  let fixed = domain 4. 4. 1e-100 in
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity ] ~f:(fun v ->
    assert (Result.is_error (N.Domain.normalize fixed v));
    assert (Result.is_error (N.Domain.advance fixed v ~direction:Increase)));
  List.iter [ -100.; 4.; 100. ] ~f:(fun v ->
    assert (Float.equal (N.Domain.normalize fixed v |> ok) 4.);
    assert (Float.equal (N.Domain.advance fixed v ~direction:Decrease |> ok) 4.));
  let wide_step = domain (-1.) 1. Float.max_finite_value in
  assert (Float.equal (N.Domain.advance wide_step (-1.) ~direction:Increase |> ok) 1.);
  assert (Float.equal (N.Domain.advance wide_step 1. ~direction:Decrease |> ok) (-1.));
  print_endline
    "invalid domains/nonfinite values rejected; fixed and larger-than-span steps saturate";
  [%expect
    {| invalid domains/nonfinite values rejected; fixed and larger-than-span steps saturate |}]
;;

let%expect_test "normalization is idempotent and steps preserve adjacency across scales" =
  List.iter
    [ 0., 1., 0.2
    ; -5., 5., 0.125
    ; 0.1, 1., 0.2
    ; 1e9, 1e9 +. 0.01, 0.00001
    ; 0., 1., 1e-12
    ; -1e200, 1e200, 1e195
    ; -1e-200, 1e-200, 1e-205
    ]
    ~f:(fun (minimum, maximum, step) ->
      let d = domain minimum maximum step in
      for i = 0 to 1024 do
        let value = minimum +. ((maximum -. minimum) *. Float.of_int i /. 1024.) in
        let value = N.Domain.normalize d value |> ok in
        assert (N.Domain.contains d value);
        assert (Float.equal (N.Domain.normalize d value |> ok) value);
        let up = N.Domain.advance d value ~direction:Increase |> ok in
        let down = N.Domain.advance d value ~direction:Decrease |> ok in
        assert (Float.(up >= value && down <= value));
        if Float.(value < maximum)
        then (
          assert (Float.(up > value));
          assert (Float.equal (N.Domain.advance d up ~direction:Decrease |> ok) value));
        if Float.(value > minimum)
        then (
          assert (Float.(down < value));
          assert (Float.equal (N.Domain.advance d down ~direction:Increase |> ok) value))
      done);
  print_endline "7175 samples: bounded, idempotent, strictly progressing and adjacent";
  [%expect {| 7175 samples: bounded, idempotent, strictly progressing and adjacent |}]
;;

let%expect_test "drafts preserve incomplete edits and distinguish validity" =
  let d = domain (-10.) 10. 0.1 in
  List.iter
    [ ""
    ; " \t"
    ; "-"
    ; "+."
    ; "."
    ; "1e"
    ; "1e-"
    ; "1."
    ; "-.5"
    ; " +2e0 "
    ; "11"
    ; "-11"
    ; "1e309"
    ; "nan"
    ; "inf"
    ; "0x10"
    ; "1_000"
    ; "1,2"
    ; "1e-x"
    ; "１２"
    ]
    ~f:(fun text -> print_s [%sexp (text : string), (N.Draft.parse d text : N.Draft.t)]);
  assert (
    N.Draft.equal
      (N.Draft.parse d (String.make 4097 '1'))
      (N.Draft.parse d (String.make 5000 '2')));
  [%expect
    {|
    ("" Empty)
    (" \t" Empty)
    (- Incomplete)
    (+. Incomplete)
    (. Incomplete)
    (1e Incomplete)
    (1e- Incomplete)
    (1. (Valid 1))
    (-.5 (Valid -0.5))
    (" +2e0 " (Valid 2))
    (11 (Out_of_range 11))
    (-11 (Out_of_range -11))
    (1e309 (Invalid Non_finite))
    (nan (Invalid Syntax))
    (inf (Invalid Syntax))
    (0x10 (Invalid Syntax))
    (1_000 (Invalid Syntax))
    (1,2 (Invalid Syntax))
    (1e-x (Invalid Syntax))
    ("\239\188\145\239\188\146" (Invalid Syntax))
  |}]
;;

let%expect_test "independent numeric domain fixture and validated import" =
  let module W = Gpuio_protocol.Numeric_wire in
  let d = domain (-1.5) 2.25 0.125 in
  let bytes =
    Bin_prot.Utils.bin_dump W.Domain.bin_writer_t (N.Expert.to_wire d)
    |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
    |> String.concat
  in
  Eio_main.run (fun env ->
    assert (
      String.equal
        hex
        (Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "numeric-domain.hex") |> String.strip)));
  let pos_ref = ref 0 in
  let imported =
    W.Domain.bin_read_t (Bigstring.of_string bytes) ~pos_ref |> N.Expert.of_wire |> ok
  in
  assert (!pos_ref = 24 && N.Domain.equal d imported);
  assert (Result.is_error (N.Expert.of_wire { min = 0.; max = 1.; step = Float.nan }));
  print_endline hex;
  [%expect {| 000000000000f8bf0000000000000240000000000000c03f |}]
;;
