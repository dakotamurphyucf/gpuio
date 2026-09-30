open Core
module S = Gpuio.Text_shimmer
module W = Gpuio_protocol.Text_shimmer_wire

let ok = Or_error.ok_exn
let encode t = Bin_prot.Utils.bin_dump W.Config.bin_writer_t t |> Bigstring.to_string

let hex bytes =
  String.to_list bytes
  |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
  |> String.concat
;;

let minimal : W.Config.t =
  { duration_ms = 1
  ; spread = Relative 0.5
  ; direction = Left_to_right
  ; repeat = Loop
  ; animated = true
  ; highlight = None
  }
;;

let explicit : W.Config.t =
  { duration_ms = 60_000
  ; spread = Pixels 128.5
  ; direction = Right_to_left
  ; repeat = Once
  ; animated = false
  ; highlight = Some 0xffff_ffffL
  }
;;

let%expect_test "independent Rust fixtures cover every variant and optional color" =
  List.iter [ minimal; explicit ] ~f:(fun fixture ->
    let config = S.Expert.of_wire fixture |> ok in
    assert (W.Config.equal fixture (S.Expert.to_wire config));
    let bytes = encode fixture in
    assert (W.Config.equal (W.Config.decode bytes |> ok) fixture);
    print_endline (hex bytes));
  print_s [%sexp (S.Expert.to_wire S.Config.default : W.Config.t)];
  [%expect
    {|
    0100000000000000e03f00010100
    fd60ea000001000000000010604001000001fcffffffff00000000
    ((duration_ms 2000) (spread (Relative 0.3)) (direction Left_to_right)
     (repeat Loop) (animated true) (highlight ()))
    |}]
;;

let%expect_test "spread and duration boundaries reject rather than clamp" =
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -1.; 0. ] ~f:(fun value ->
    assert (Result.is_error (S.Spread.relative value));
    assert (Result.is_error (S.Spread.pixels value)));
  List.iter [ 0.049; 1.001 ] ~f:(fun value ->
    assert (Result.is_error (S.Spread.relative value)));
  List.iter [ 0.999; 1_000_001. ] ~f:(fun value ->
    assert (Result.is_error (S.Spread.pixels value)));
  List.iter [ 0.05; 1. ] ~f:(fun value -> assert (Result.is_ok (S.Spread.relative value)));
  List.iter [ 1.; 1_000_000. ] ~f:(fun value ->
    assert (Result.is_ok (S.Spread.pixels value)));
  List.iter [ -1.; 0.; 0.999; 60_000.001 ] ~f:(fun ms ->
    assert (Result.is_error (S.Config.create ~duration:(Time_ns.Span.of_ms ms) ())));
  List.iter [ 1.; 1.000_001; 59_999.5; 60_000. ] ~f:(fun ms ->
    let t = S.Config.create ~duration:(Time_ns.Span.of_ms ms) () |> ok in
    print_s [%sexp ((S.Expert.to_wire t).duration_ms : int)]);
  assert (S.Spread.equal S.Spread.default (S.Spread.relative 0.3 |> ok));
  [%expect
    {|
    1
    2
    60000
    60000
    |}]
;;

let%expect_test "highlight resolution is optional and preserves explicit alpha" =
  let empty = Gpuio.Theme.create [] |> ok in
  assert (S.Config.equal (S.Config.create ~theme:empty () |> ok) S.Config.default);
  let color = Gpuio.Color.rgba ~red:1 ~green:2 ~blue:3 ~alpha:128 |> ok in
  let theme = Gpuio.Theme.create [ "shimmer", color ] |> ok in
  let direct = S.Config.create ~theme:empty ~highlight:color () |> ok in
  let token =
    S.Config.create ~theme ~highlight:(Gpuio.Color.token_exn "shimmer") () |> ok
  in
  assert (S.Config.equal direct token);
  assert (
    Result.is_error
      (S.Config.create ~theme:empty ~highlight:(Gpuio.Color.token_exn "shimmer") ()));
  let transparent =
    S.Config.create
      ~highlight:(Gpuio.Color.rgba ~red:255 ~green:255 ~blue:255 ~alpha:0 |> ok)
      ()
    |> ok
  in
  print_s
    [%sexp
      ((S.Expert.to_wire token).highlight : int64 option)
    , ((S.Expert.to_wire transparent).highlight : int64 option)];
  [%expect {| ((16909184) (4294967040)) |}]
;;

let%expect_test "raw invalid configs and malformed bytes cannot bypass validation" =
  let reject t =
    assert (not (W.Config.valid t));
    assert (Result.is_error (S.Expert.of_wire t));
    assert (Result.is_error (W.Config.decode (encode t)))
  in
  List.iter [ Int.min_value; -1; 0; 60_001; Int.max_value ] ~f:(fun duration_ms ->
    reject { minimal with duration_ms });
  List.iter [ -1L; 0x1_0000_0000L; Int64.max_value ] ~f:(fun color ->
    reject { minimal with highlight = Some color });
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -1.; 0.; 0.049; 1.001 ]
    ~f:(fun value -> reject { minimal with spread = Relative value });
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -1.; 0.; 0.999; 1_000_001. ]
    ~f:(fun value -> reject { minimal with spread = Pixels value });
  List.iter [ minimal; explicit ] ~f:(fun t ->
    let bytes = encode t in
    for length = 0 to String.length bytes - 1 do
      assert (Result.is_error (W.Config.decode (String.prefix bytes length)))
    done;
    assert (Result.is_error (W.Config.decode (bytes ^ "\000"))));
  let bytes = encode minimal in
  (* Spread, direction, repeat, Boolean, option. All are strict one-byte tags. *)
  List.iter [ 1; 10; 11; 12; 13 ] ~f:(fun index ->
    List.iter [ 2; 255 ] ~f:(fun tag ->
      let malformed = Bytes.of_string bytes in
      Bytes.set malformed index (Char.of_int_exn tag);
      assert (Result.is_error (W.Config.decode (Bytes.to_string malformed)))));
  assert (Result.is_error (W.Config.decode (String.make (W.max_config_bytes + 1) '\000')));
  print_endline "invalid domains, every truncation, trailing data, tags and size rejected";
  [%expect {| invalid domains, every truncation, trailing data, tags and size rejected |}]
;;

let%expect_test "cross product bounds encoded and retained configuration size" =
  let count = ref 0 in
  List.iter [ 1; 127; 128; 32_767; 32_768; 60_000 ] ~f:(fun duration_ms ->
    List.iter
      [ W.Spread.Relative 0.05; Relative 1.; Pixels 1.; Pixels 1_000_000. ]
      ~f:(fun spread ->
        List.iter [ W.Direction.Left_to_right; Right_to_left ] ~f:(fun direction ->
          List.iter [ W.Repeat.Once; Loop ] ~f:(fun repeat ->
            List.iter [ false; true ] ~f:(fun animated ->
              List.iter [ None; Some 0L; Some 0xffff_ffffL ] ~f:(fun highlight ->
                let t : W.Config.t =
                  { duration_ms; spread; direction; repeat; animated; highlight }
                in
                let bytes = encode t in
                assert (String.length bytes <= W.max_config_bytes);
                assert (W.Config.equal (W.Config.decode bytes |> ok) t);
                Int.incr count))))));
  print_s [%sexp (!count : int)];
  [%expect {| 576 |}]
;;
