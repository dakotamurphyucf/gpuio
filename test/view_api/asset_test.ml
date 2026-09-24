open Core
module Asset = Gpuio.Asset

let%expect_test "opaque data and bounded diagnostics" =
  let data = "\000\255\128X" in
  let source = Asset.Source.of_bytes ~format:Png data |> Or_error.ok_exn in
  print_s [%sexp (source : Asset.Source.t)];
  print_s
    [%sexp
      { unchanged = (String.equal data (Asset.Source.bytes source) : bool)
      ; format = (Asset.Source.format source : Asset.Format.t)
      ; length = (Asset.Source.byte_length source : int)
      ; different_format =
          (not
             (Asset.Source.equal
                source
                (Asset.Source.of_bytes ~format:Svg data |> Or_error.ok_exn))
           : bool)
      }];
  [%expect
    {| 
    ((format Png) (byte_length 4))
    ((unchanged true) (format Png) (length 4) (different_format true))
  |}]
;;

let%expect_test "encoded limit is independent of transport envelope size" =
  List.iter
    [ 0; 1; Asset.Source.max_bytes; Asset.Source.max_bytes + 1 ]
    ~f:(fun length ->
      let result = Asset.Source.of_bytes ~format:Png (String.make length '\000') in
      print_s [%sexp (length : int), (Result.is_ok result : bool)]);
  [%expect
    {|
    (0 false)
    (1 true)
    (16777216 true)
    (16777217 false)
  |}]
;;
