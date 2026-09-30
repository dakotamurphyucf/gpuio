open Core
open Gpuio

let ok = Or_error.ok_exn
let tint color factor = Color.with_opacity color factor |> ok
let empty = Theme.create [] |> ok

let%expect_test "opacity resolves the current theme, preserves RGB and rounds once" =
  let token = Color.token_exn "status" in
  let transparent = tint token 0. in
  let tinted = tint (tint token 0.5) 0.5 in
  List.iter [ 1; 128; 255 ] ~f:(fun alpha ->
    let original = Color.rgba ~red:18 ~green:52 ~blue:86 ~alpha |> ok in
    let theme = Theme.create [ "status", original ] |> ok in
    printf
      "%08Lx %08Lx %08Lx\n"
      (Theme.resolve theme tinted |> ok)
      (Theme.resolve theme transparent |> ok)
      (Theme.resolve theme (tint token 1.) |> ok));
  assert (Result.is_error (Theme.resolve empty transparent));
  let concrete = tint (Color.rgb_exn 0x123456) 0.5 in
  let defined = Theme.create [ "status", concrete ] |> ok in
  assert (Int64.equal (Theme.resolve defined token |> ok) 0x12345680L);
  assert (Result.is_error (Theme.create [ "status", transparent ]));
  let styles =
    Style.create_exn [ Foreground tinted; Background (Background.solid transparent) ]
  in
  assert (Result.is_error (Style.Expert.to_wire styles ~theme:empty));
  ignore
    (Style.Expert.to_wire styles ~theme:defined |> ok : Gpuio_protocol.Wire.Style.t list);
  [%expect
    {|
    12345600 12345600 12345601
    12345620 12345600 12345680
    12345640 12345600 123456ff
    |}]
;;

let%expect_test "opacity rejects invalid factors and keeps repeated updates bounded" =
  let color = Color.rgb_exn 0x123456 in
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -0.001; 1.001 ]
    ~f:(fun factor -> assert (Result.is_error (Color.with_opacity color factor)));
  assert (Color.equal color (tint color 1.));
  let repeated = ref color in
  for _ = 1 to 100_000 do
    repeated := tint !repeated 0.999
  done;
  (match Color.Expert.value !repeated with
   | Opacity (base, _) -> assert (Color.equal base color)
   | Rgba _ | Token _ -> assert false);
  assert (Int64.equal (Theme.resolve empty !repeated |> ok) 0x12345600L);
  print_endline
    "invalid factors rejected; 100000 updates keep one expression and original RGB";
  [%expect
    {| invalid factors rejected; 100000 updates keep one expression and original RGB |}]
;;
