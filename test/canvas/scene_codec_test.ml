open Core
module Geometry = Gpuio_protocol.Canvas_wire
module Scene = Gpuio_protocol.Canvas_scene_wire

let fixture () : Scene.t =
  let point x y : Geometry.Point.t = { x; y } in
  let rect : Geometry.Rect.t = { x = 0.; y = 0.; width = 20.; height = 10. } in
  let key id generation : Scene.Resource_key.t = { id; generation } in
  let paint : Scene.Paint.t =
    { fill = Some 0x102030ffL; stroke = Some { color = 0xff0000ffL; width = 2. } }
  in
  let identity = Geometry.Transform.identity in
  { version = 1L
  ; description = "Diagram 🦀"
  ; resources =
      [ { key = key 1L 2L
        ; data =
            Path [ Move (point 0. 0.); Line (point 10. 10.); Line (point 0. 10.); Close ]
        }
      ; { key = key 2L 1L
        ; data =
            Text
              { value = "héllo 🦀"
              ; font_family = "system"
              ; font_size = 14.
              ; font_weight = 500L
              }
        }
      ; { key = key 3L 3L
        ; data =
            Image
              (Gpuio_protocol.Resource_id.create ~slot:7L ~generation:2L
               |> Or_error.ok_exn)
        }
      ]
  ; items =
      [ { id = 1L
        ; transform = identity
        ; clips = [ rect ]
        ; drawing = Shape (Rectangle rect, paint)
        ; interaction =
            Some
              { label = "Rectangle"
              ; hit_region = Rectangle rect
              ; draggable = true
              ; activatable = true
              }
        }
      ; { id = 2L
        ; transform = { identity with tx = 30. }
        ; clips = []
        ; drawing = Shape (Ellipse rect, { fill = Some 0xff00ffffL; stroke = None })
        ; interaction =
            Some
              { label = "Ellipse"
              ; hit_region = Ellipse rect
              ; draggable = false
              ; activatable = true
              }
        }
      ; { id = 3L
        ; transform = { identity with ty = 30. }
        ; clips = []
        ; drawing = Shape (Path (key 1L 2L), { fill = None; stroke = paint.stroke })
        ; interaction =
            Some
              { label = "Path"
              ; hit_region = Polygon [ point 0. 0.; point 10. 10.; point 0. 10. ]
              ; draggable = true
              ; activatable = false
              }
        }
      ; { id = 4L
        ; transform = identity
        ; clips = []
        ; drawing = Text (key 2L 1L, point 10. 50., 0xffffffffL)
        ; interaction = None
        }
      ; { id = 5L
        ; transform = identity
        ; clips = []
        ; drawing = Image (key 3L 3L, rect)
        ; interaction = None
        }
      ]
  }
;;

let%expect_test "scene fixture preserves resource identities, all drawing kinds and order"
  =
  Eio_main.run (fun env ->
    let hex =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "canvas-v1-scene.hex") |> String.strip
    in
    let bytes =
      String.init
        (String.length hex / 2)
        ~f:(fun index ->
          Char.of_int_exn (Int.of_string ("0x" ^ String.sub hex ~pos:(index * 2) ~len:2)))
    in
    let scene = fixture () in
    let encoded =
      Bin_prot.Utils.bin_dump Scene.bin_writer_t scene |> Bigstring.to_string
    in
    assert (String.equal encoded bytes);
    let position = ref 0 in
    let decoded = Scene.bin_read_t (Bigstring.of_string bytes) ~pos_ref:position in
    assert (!position = String.length bytes);
    assert (Scene.equal scene decoded);
    print_s [%sexp (List.map decoded.items ~f:(fun item -> item.id) : int64 list)];
    print_endline "independent OCaml/Rust scene fixture agrees");
  [%expect
    {|
    (1 2 3 4 5)
    independent OCaml/Rust scene fixture agrees
    |}]
;;
