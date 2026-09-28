open Core
module State = Gpuio_gallery_model.Numeric_state
module Rating = Gpuio.Rating

let%expect_test "canvas moves preserve other shapes and reject invalid whole scenes" =
  let module Study = Gpuio_gallery_model.Canvas_study in
  let module G = Gpuio.Canvas_geometry in
  let ok = Or_error.ok_exn in
  let orbit = Study.items Study.initial |> List.hd_exn |> Study.Item.id in
  let moved =
    Study.move Study.initial orbit (G.Transform.translate ~x:180. ~y:144. |> ok) |> ok
  in
  let positions model =
    List.map (Study.items model) ~f:(fun item ->
      let point = Study.Item.position item in
      Study.Item.name item, (G.Point.x point, G.Point.y point))
  in
  print_s [%sexp (positions moved : (string * (float * float)) list)];
  print_s
    [%sexp
      (Or_error.is_error
         (Study.move
            moved
            (Gpuio.Canvas_scene.Item_id.of_int64 999L |> ok)
            G.Transform.identity)
       : bool)
    , (Or_error.is_error
         (Study.move moved orbit (G.Transform.translate ~x:1_000_000. ~y:0. |> ok))
       : bool)
    , (Or_error.is_error (Study.move moved orbit (G.Transform.rotate ~radians:0.5 |> ok))
       : bool)];
  print_s [%sexp (positions Study.initial : (string * (float * float)) list)];
  [%expect
    {|
    ((Orbit (180 144)) (Prism (290 100)) (Tile (470 160)))
    (true true true)
    ((Orbit (110 130)) (Prism (290 100)) (Tile (470 160)))
    |}]
;;

let%expect_test "read-only fences delayed requests and relative bursts saturate" =
  let apply state actions = List.fold actions ~init:state ~f:State.apply in
  let show state =
    print_s
      [%sexp
        (State.is_read_only state : bool)
      , (Rating.Config.value (State.rating state) : int)]
  in
  let locked =
    apply
      State.initial
      [ Toggle_read_only; Rate Rating.Request.increase; Rate Rating.Request.clear ]
  in
  show locked;
  let unlocked =
    apply
      locked
      [ Toggle_read_only
      ; Rate Rating.Request.increase
      ; Rate Rating.Request.increase
      ; Rate Rating.Request.increase
      ]
  in
  show unlocked;
  let cleared =
    State.apply unlocked (Rate (Rating.Request.toggle 5 |> Or_error.ok_exn))
  in
  show cleared;
  [%expect
    {|
    (true 3)
    (false 5)
    (false 0)
  |}]
;;

let%expect_test "commands use current availability; old dismissals preserve a replacement"
  =
  let module F = Gpuio_gallery_model.Feedback_state in
  let apply state actions = List.fold actions ~init:state ~f:F.apply in
  let show state =
    print_s
      [%sexp
        (F.stage state : F.Stage.t)
      , (F.is_enabled state : bool)
      , (F.notification state : int option)]
  in
  let disabled = apply F.initial [ Toggle_enabled; Advance ] in
  show disabled;
  let working = apply disabled [ Toggle_enabled; Advance; Notify ] in
  show working;
  let replacement = apply working [ Notify; Dismiss 1; Advance; Advance ] in
  show replacement;
  show (apply replacement [ Dismiss 2; Dismiss 2; Advance ]);
  show (apply replacement [ Leave; Notify; Dismiss 2 ]);
  [%expect
    {|
    (Idle false ())
    (Working true (1))
    (Complete true (2))
    (Idle true ())
    (Complete true (3))
  |}]
;;
