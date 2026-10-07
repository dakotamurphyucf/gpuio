open Core
open Gpuio
module W = Gpuio_protocol.Wire

let ok = Or_error.ok_exn
let color = Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> ok
let hex bytes = String.concat_map bytes ~f:(fun c -> sprintf "%02x" (Char.to_int c))

let%expect_test "pattern dimensions reject nonfinite and unsafe native packing inputs" =
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; 0.; 0.49; 64.01 ]
    ~f:(fun n ->
      assert (Result.is_error (Background.pattern_slash color ~width:n ~interval:4.));
      assert (Result.is_error (Background.pattern_slash color ~width:2. ~interval:n));
      assert (Result.is_error (Background.checkerboard color ~size:n)));
  List.iter [ 0.5; 64. ] ~f:(fun n ->
    ignore (Background.pattern_slash color ~width:n ~interval:n |> ok : Background.t);
    ignore (Background.checkerboard color ~size:n |> ok : Background.t));
  print_endline "invalid dimensions rejected; both inclusive endpoints accepted";
  [%expect {| invalid dimensions rejected; both inclusive endpoints accepted |}]
;;

let%expect_test "pattern fill bytes are independently paired with Rust and theme checked" =
  List.iter
    [ Background.pattern_slash color ~width:2. ~interval:4. |> ok
    ; Background.checkerboard color ~size:8. |> ok
    ]
    ~f:(fun background ->
      let wire = Style.Expert.background_to_wire background ~theme:Theme.default |> ok in
      print_endline
        (Bin_prot.Utils.bin_dump W.Fill.bin_writer_t wire |> Bigstring.to_string |> hex));
  let token = Color.token_exn "pattern" in
  let background = Background.checkerboard token ~size:8. |> ok in
  assert (
    Result.is_error (Style.Expert.background_to_wire background ~theme:Theme.default));
  let theme = Theme.create [ "pattern", color ] |> ok in
  let wire = Style.Expert.background_to_wire background ~theme |> ok in
  assert (W.Fill.equal wire (Checkerboard (Rgba 0L, 8.)));
  [%expect
    {|
    03000000000000000000400000000000001040
    0400000000000000002040
    |}]
;;
