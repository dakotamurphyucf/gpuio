open Core
open Gpuio_protocol

let request =
  let node slot = Node_id.create ~slot ~generation:2L |> Or_error.ok_exn in
  Wire.Message.Apply
    { window = Window_id.create ~slot:3L ~generation:2L |> Or_error.ok_exn
    ; base = 127L
    ; revision = 128L
    ; operations =
        [ Create
            ( node 4L
            , Checkbox
            , "界"
            , Some (Handler_id.create ~slot:5L ~generation:4L |> Or_error.ok_exn) )
        ; Create (node 6L, Switch, "", None)
        ; Set_control (node 4L, Checkbox (Unchecked, false))
        ; Set_control (node 4L, Checkbox (Checked, true))
        ; Set_control (node 4L, Checkbox (Indeterminate, false))
        ; Set_control (node 6L, Switch (true, false))
        ; Set_control (node 6L, Switch (false, true))
        ; Set_control (node 7L, Button false)
        ; Set_control (node 7L, Button true)
        ; Set_style
            ( node 4L
            , List.init 4 ~f:(fun i ->
                Wire.Style.State (Int64.of_int (i + 4), [ Opacity 0.5 ])) )
        ]
    }
;;

let%expect_test "control and native semantic style tags match the Rust fixture" =
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "controls-v1.hex") |> String.strip
    in
    let bytes = Wire.Message.encode request |> Or_error.ok_exn in
    let actual =
      String.to_list bytes
      |> List.map ~f:(fun c -> sprintf "%02x" (Char.to_int c))
      |> String.concat
    in
    assert (String.equal actual expected);
    print_endline "CONTROL_CODEC_PASS");
  [%expect {| CONTROL_CODEC_PASS |}]
;;
