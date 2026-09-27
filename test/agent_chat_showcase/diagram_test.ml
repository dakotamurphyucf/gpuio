open Core
module Diagram = Gpuio_agent_chat_runtime.Run_diagram
module Stage = Diagram.Stage
module G = Gpuio.Canvas_geometry

let ok = Or_error.ok_exn

let%expect_test "moving a stage preserves the other stages and publishes a bounded scene" =
  let original = Diagram.create () in
  let moved =
    Diagram.move original Read (G.Transform.translate ~x:55. ~y:52. |> ok) |> ok
  in
  List.iter Stage.all ~f:(fun stage ->
    let point = Diagram.position moved stage in
    print_s
      [%sexp (stage : Stage.t), (G.Point.x point : float), (G.Point.y point : float)]);
  print_s [%sexp (G.Point.x (Diagram.position original Read) : float)];
  List.iter [ true; false ] ~f:(fun dark ->
    let scene =
      Diagram.scene
        moved
        ~palette:(Gpuio_agent_chat_runtime.Palette.of_dark dark)
        ~generation:2L
    in
    print_s
      [%sexp
        (Gpuio.Canvas_scene.item_count scene : int)
      , (Gpuio.Canvas_scene.resource_count scene : int)]);
  [%expect
    {| 
    (Read 55 52)
    (Draft 34 136)
    (Review 34 230)
    34
    (5 5)
    (5 5) |}]
;;

let%expect_test "reject transforms that would invalidate text or connector geometry" =
  List.iter
    [ G.Transform.rotate ~radians:0.5
    ; G.Transform.translate ~x:999_999. ~y:0.
    ; G.Transform.translate ~x:0. ~y:(-999_999.)
    ]
    ~f:(fun transform ->
      print_s
        [%sexp
          (Diagram.move (Diagram.create ()) Read (ok transform) |> Result.is_error : bool)]);
  List.iter Stage.all ~f:(fun stage ->
    print_s
      [%sexp
        (Option.equal Stage.equal (Some stage) (Stage.of_id (Stage.id stage)) : bool)]);
  [%expect
    {|
    true
    true
    true
    true
    true
    true
    |}]
;;

let%expect_test "confirmed annotation changes both connector paints and preserves alpha" =
  let module Wire = Gpuio_protocol.Canvas_scene_wire in
  let strokes ~dark annotation =
    let scene =
      Diagram.scene
        ~annotation
        (Diagram.create ())
        ~palette:(Gpuio_agent_chat_runtime.Palette.of_dark dark)
        ~generation:3L
    in
    let bytes =
      Gpuio.Canvas_scene.Expert.encode
        scene
        ~asset_owner:(Gpuio.Asset.Expert.Owner.create ())
      |> ok
    in
    let wire = Wire.bin_read_t (Bigstring.of_string bytes) ~pos_ref:(ref 0) in
    List.filter_map wire.items ~f:(fun item ->
      match item.drawing with
      | Shape (_, paint) -> Option.map paint.stroke ~f:(fun stroke -> stroke.color)
      | Text _ | Image _ -> None)
  in
  let custom =
    Gpuio.Color_value.Value.Color (Gpuio.Color_value.Rgba.of_hex "#FF000080" |> ok)
  in
  List.iter [ true; false ] ~f:(fun dark ->
    print_s [%sexp (List.map (strokes ~dark custom) ~f:Int64.Hex.to_string : string list)]);
  print_s
    [%sexp
      (not (List.equal Int64.equal (strokes ~dark:true Empty) (strokes ~dark:false Empty))
       : bool)];
  [%expect
    {|
    (0xff000080 0xff000080)
    (0xff000080 0xff000080)
    true
    |}]
;;
