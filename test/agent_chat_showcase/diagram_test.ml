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
