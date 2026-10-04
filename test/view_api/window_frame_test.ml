open Core
open Gpuio

let%expect_test "window frames validate finite native geometry" =
  List.iter
    [ 0., 0.5; 20., 4.; 128., 32. ]
    ~f:(fun (shadow_size, resize_hit_size) ->
      let frame =
        Window_frame.create ~shadow_size ~resize_hit_size () |> Or_error.ok_exn
      in
      let wire = Window_frame.Expert.to_wire frame in
      assert (Float.equal wire.shadow_size shadow_size);
      assert (Float.equal wire.resize_hit_size resize_hit_size));
  List.iter
    [ -1., 4.
    ; 129., 4.
    ; Float.nan, 4.
    ; Float.infinity, 4.
    ; 20., 0.
    ; 20., 33.
    ; 20., Float.nan
    ; 20., Float.neg_infinity
    ]
    ~f:(fun (shadow_size, resize_hit_size) ->
      assert (Result.is_error (Window_frame.create ~shadow_size ~resize_hit_size ()));
      let config : Gpuio_protocol.Window_wire.Config.t =
        { title = "Frame"
        ; width = 640.
        ; height = 400.
        ; focus = false
        ; chrome = Custom
        ; resizable = true
        ; frame = { shadow_size; resize_hit_size }
        }
      in
      let id =
        Gpuio_protocol.Window_id.create ~slot:0L ~generation:1L |> Or_error.ok_exn
      in
      assert (
        Result.is_error
          (Gpuio_protocol.Wire.Message.encode (Open_configured (7L, id, config)))));
  print_endline "finite boundaries accepted; invalid API and wire geometry rejected";
  [%expect {| finite boundaries accepted; invalid API and wire geometry rejected |}]
;;
