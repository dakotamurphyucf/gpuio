open Core
open Gpuio
module C = Color_value

let ok = Or_error.ok_exn
let rgba red green blue alpha = C.Rgba.create ~red ~green ~blue ~alpha |> ok

let hsla hue_degrees saturation lightness alpha =
  C.Hsla.create ~hue_degrees ~saturation ~lightness ~alpha |> ok
;;

let near a b = Float.(abs (a -. b) <= 1e-10)

let%expect_test "independent encoded-sRGB color reference vectors" =
  Eio_main.run (fun env ->
    let lines =
      Eio.Path.load Eio.Path.(Eio.Stdenv.fs env / "color-values.tsv")
      |> String.split_lines
    in
    List.iter lines ~f:(fun line ->
      match String.split line ~on:'\t' with
      | [ r; g; b; a; h; s; l; opacity ] ->
        let color =
          rgba (Int.of_string r) (Int.of_string g) (Int.of_string b) (Int.of_string a)
        in
        let expected =
          hsla
            (Float.of_string h)
            (Float.of_string s)
            (Float.of_string l)
            (Float.of_string opacity)
        in
        let actual = C.Rgba.to_hsla color in
        List.iter
          [ C.Hsla.hue_degrees; C.Hsla.saturation; C.Hsla.lightness; C.Hsla.alpha ]
          ~f:(fun channel -> assert (near (channel actual) (channel expected)));
        assert (C.Rgba.equal (C.Rgba.of_hsla expected) color)
      | _ -> assert false);
    printf
      "%d independent RGBA/HSLA vectors agree in both directions\n"
      (List.length lines));
  [%expect {| 12 independent RGBA/HSLA vectors agree in both directions |}]
;;

let%expect_test "sampled byte colors roundtrip without drift, including transparency" =
  let count = ref 0 in
  for red = 0 to 15 do
    for green = 0 to 15 do
      for blue = 0 to 15 do
        List.iter [ 0; 1; 64; 127; 128; 255 ] ~f:(fun alpha ->
          let color = rgba (red * 17) (green * 17) (blue * 17) alpha in
          assert (C.Rgba.equal color (C.Rgba.of_hsla (C.Rgba.to_hsla color)));
          assert (C.Rgba.equal color (C.Rgba.of_hex (C.Rgba.to_hex color) |> ok));
          Int.incr count)
      done
    done
  done;
  printf "%d byte colors: exact HSLA and hex roundtrips\n" !count;
  [%expect {| 24576 byte colors: exact HSLA and hex roundtrips |}]
;;

let%expect_test "finite ranges, hue wrap and half-up quantization" =
  List.iter
    [ Float.nan; Float.infinity; Float.neg_infinity; -0.1; 360.1 ]
    ~f:(fun hue_degrees ->
      assert (
        Result.is_error
          (C.Hsla.create ~hue_degrees ~saturation:1. ~lightness:0.5 ~alpha:1.)));
  List.iter [ Float.nan; Float.infinity; Float.neg_infinity; -0.1; 1.1 ] ~f:(fun x ->
    List.iter
      [ x, 0.5, 1.; 1., x, 1.; 1., 0.5, x ]
      ~f:(fun (saturation, lightness, alpha) ->
        assert (
          Result.is_error (C.Hsla.create ~hue_degrees:0. ~saturation ~lightness ~alpha))));
  List.iter [ -1; 256; Int.max_value; Int.min_value ] ~f:(fun x ->
    List.iter
      [ x, 0, 0, 0; 0, x, 0, 0; 0, 0, x, 0; 0, 0, 0, x ]
      ~f:(fun (red, green, blue, alpha) ->
        assert (Result.is_error (C.Rgba.create ~red ~green ~blue ~alpha))));
  assert (C.Hsla.equal (hsla 360. 1. 0.5 1.) (hsla 0. 1. 0.5 1.));
  assert (C.Hsla.equal (hsla (-0.) (-0.) (-0.) (-0.)) (hsla 0. 0. 0. 0.));
  for i = 0 to 10000 do
    let value = Float.of_int i /. 10000. in
    let color = C.Rgba.of_hsla (hsla 123. 0. value value) in
    let error = Float.abs ((Float.of_int (C.Rgba.red color) /. 255.) -. value) in
    assert (Float.(error <= (0.5 /. 255.) +. 1e-15));
    assert (
      C.Rgba.red color = C.Rgba.green color
      && C.Rgba.red color = C.Rgba.blue color
      && C.Rgba.red color = C.Rgba.alpha color)
  done;
  List.iter [ 0.; 60.; 120.; 180.; 240.; 300.; 360. ] ~f:(fun hue ->
    print_endline (C.Rgba.to_hex (C.Rgba.of_hsla (hsla hue 1. 0.5 0.5))));
  [%expect
    {|
    #FF000080
    #FFFF0080
    #00FF0080
    #00FFFF80
    #0000FF80
    #FF00FF80
    #FF000080
  |}]
;;

let%expect_test
    "hex draft validation preserves incomplete edits and rejects non-ASCII syntax"
  =
  let classify text =
    match C.Hex_draft.parse text with
    | Empty -> "empty"
    | Incomplete -> "incomplete"
    | Invalid Syntax -> "syntax"
    | Invalid Too_long -> "too long"
    | Valid value -> C.Rgba.to_hex value
  in
  List.iter
    [ ""
    ; "#"
    ; "f"
    ; "12"
    ; "12345"
    ; "1234567"
    ; "#abc"
    ; "AbC0"
    ; "#123456"
    ; "11223344"
    ; "#FFFFFFFF"
    ; " #123"
    ; "#123 "
    ; "+123"
    ; "0x12"
    ; "##123"
    ; "ｆｆ"
    ; "123456789"
    ; "#123456789"
    ]
    ~f:(fun text -> printf "%s\n" (classify text));
  assert (Result.is_error (C.Rgba.of_hex ""));
  assert (String.equal (classify (String.make 100000 'a')) "too long");
  let transparent = rgba 0 0 0 0 in
  assert (not (C.Value.equal Empty (Color transparent)));
  assert (
    Color.equal
      (C.Rgba.to_color transparent)
      (Color.rgba ~red:0 ~green:0 ~blue:0 ~alpha:0 |> ok));
  [%expect
    {|
    empty
    incomplete
    incomplete
    incomplete
    incomplete
    incomplete
    #AABBCC
    #AABBCC00
    #123456
    #11223344
    #FFFFFF
    syntax
    syntax
    syntax
    syntax
    syntax
    syntax
    too long
    too long
  |}]
;;
