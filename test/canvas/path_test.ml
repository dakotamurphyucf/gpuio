open Core
module Path = Gpuio.Canvas_path
open Path.Command

let point x y = Gpuio.Canvas_geometry.Point.create ~x ~y |> Or_error.ok_exn
let origin = point 0. 0.
let endpoint = point 10. 10.

let%expect_test "all path commands have a shared OCaml/Rust bin_prot fixture" =
  let path =
    Path.create
      [ Move origin
      ; Line (point 1. 2.)
      ; Quadratic { control = point 3. 4.; endpoint = point 5. 6. }
      ; Cubic
          { first_control = point 7. 8.
          ; second_control = point 9. 10.
          ; endpoint = point 11. 12.
          }
      ; Close
      ]
    |> Or_error.ok_exn
    |> Path.Expert.to_wire
  in
  let bytes = Bin_prot.Utils.bin_dump Gpuio_protocol.Canvas_wire.Path.bin_writer_t path in
  let position = ref 0 in
  let decoded = Gpuio_protocol.Canvas_wire.Path.bin_read_t bytes ~pos_ref:position in
  assert (Gpuio_protocol.Canvas_wire.Path.equal path decoded);
  assert (!position = Bigstring.length bytes);
  print_endline
    (Bigstring.to_string bytes
     |> String.to_list
     |> List.map ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
     |> String.concat);
  [%expect
    {| 05000000000000000000000000000000000001000000000000f03f0000000000000040020000000000000840000000000000104000000000000014400000000000001840030000000000001c400000000000002040000000000000224000000000000024400000000000002640000000000000284004 |}]
;;

let%expect_test "path topology distinguishes open strokes from filled contours" =
  let paths =
    [ [ Move origin; Line endpoint ]
    ; [ Move origin; Line endpoint; Close ]
    ; [ Move origin; Quadratic { control = point 10. 0.; endpoint }; Close ]
    ; [ Move origin
      ; Cubic { first_control = point 10. 0.; second_control = point 0. 10.; endpoint }
      ; Close
      ]
    ; [ Move origin; Line endpoint; Move endpoint; Line origin; Close ]
    ; [ Move origin; Line endpoint; Close; Move endpoint; Line origin; Close ]
    ]
  in
  print_s
    [%sexp
      (List.map paths ~f:(fun commands ->
         let path = Path.create commands |> Or_error.ok_exn in
         Path.command_count path, Path.is_closed path)
       : (int * bool) list)];
  [%expect {| ((2 false) (3 true) (3 true) (3 true) (5 false) (6 true)) |}]
;;

let%expect_test "malformed contours and excessive commands fail before native admission" =
  let paths =
    [ []
    ; [ Move origin ]
    ; [ Line endpoint ]
    ; [ Close ]
    ; [ Move origin; Close ]
    ; [ Move origin; Move endpoint; Line origin ]
    ; [ Move origin; Line endpoint; Close; Line origin ]
    ; [ Move origin; Line endpoint; Close; Close ]
    ; [ Move origin; Line endpoint; Move origin ]
    ; Move origin :: List.init 4096 ~f:(fun _ -> Line endpoint)
    ]
  in
  print_s
    [%sexp
      (List.map paths ~f:(fun commands -> Path.create commands |> Or_error.is_error)
       : bool list)];
  let maximum =
    Path.create (Move origin :: List.init 4095 ~f:(fun _ -> Line endpoint))
    |> Or_error.ok_exn
  in
  print_s [%sexp (Path.command_count maximum : int)];
  [%expect
    {|
    (true true true true true true true true true true)
    4096
    |}]
;;
