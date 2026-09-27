open Core
open Gpuio.Canvas_geometry

let point x y = Point.create ~x ~y |> Or_error.ok_exn
let rect x y width height = Rect.create ~x ~y ~width ~height |> Or_error.ok_exn
let translation x y = Transform.translate ~x ~y |> Or_error.ok_exn
let scale x y = Transform.scale ~x ~y |> Or_error.ok_exn
let compose t local = Transform.compose t ~local |> Or_error.ok_exn
let xy t = Point.x t, Point.y t

let%expect_test "invalid values and calculated results cannot enter geometry" =
  let errors =
    [ Point.create ~x:Float.nan ~y:0. |> Or_error.is_error
    ; Point.create ~x:Float.infinity ~y:0. |> Or_error.is_error
    ; Point.create ~x:1_000_001. ~y:0. |> Or_error.is_error
    ; Rect.create ~x:0. ~y:0. ~width:0. ~height:1. |> Or_error.is_error
    ; Rect.create ~x:999_999. ~y:0. ~width:2. ~height:1. |> Or_error.is_error
    ; Transform.scale ~x:0. ~y:1. |> Or_error.is_error
    ; Transform.scale ~x:1e-5 ~y:1e-5 |> Or_error.is_error
    ; Transform.rotate ~radians:Float.nan |> Or_error.is_error
    ; Transform.compose (scale 1000. 1.) ~local:(scale 2. 1.) |> Or_error.is_error
    ; Transform.apply (translation 1_000_000. 0.) (point 1. 0.) |> Or_error.is_error
    ; Transform.unapply (scale 0.0001 1.) (point 1000. 0.) |> Or_error.is_error
    ]
  in
  print_s [%sexp (errors : bool list)];
  [%expect {| (true true true true true true true true true true true) |}]
;;

let%expect_test
    "composition applies local first; inverse supports reflection and rotation"
  =
  let reflection = compose (translation 100. 50.) (scale (-2.) 3.) in
  let transformed = Transform.apply reflection (point 4. 5.) |> Or_error.ok_exn in
  let recovered = Transform.unapply reflection transformed |> Or_error.ok_exn in
  print_s [%sexp (xy transformed : float * float), (xy recovered : float * float)];
  let rotation = Transform.rotate ~radians:(Float.pi /. 2.) |> Or_error.ok_exn in
  let transformed =
    Transform.apply (compose (translation 10. 20.) rotation) (point 4. 0.)
    |> Or_error.ok_exn
  in
  print_s
    [%sexp
      (Float.(abs (Point.x transformed -. 10.) < 1e-10) : bool)
    , (Float.(abs (Point.y transformed -. 24.) < 1e-10) : bool)];
  let tiny = scale 0.0001 1. in
  print_s
    [%sexp
      (Transform.unapply tiny (point 0.0002 3.) |> Or_error.ok_exn |> xy : float * float)];
  [%expect
    {|
    ((92 65) (4 5))
    (true true)
    (2 3)
    |}]
;;

let%expect_test "world clips intersect before inverse-transformed local containment" =
  let region = Hit_region.rectangle (rect 0. 0. 10. 10.) in
  let transform = compose (translation 100. 50.) (scale (-2.) 3.) in
  let points =
    List.map
      [ 80., 50.; 100., 80.; 90., 65.; 79., 65.; 101., 65. ]
      ~f:(fun (x, y) -> point x y)
  in
  print_s
    [%sexp (List.map points ~f:(Hit_region.hit region ~transform ~clips:[]) : bool list)];
  print_s
    [%sexp
      (List.map
         points
         ~f:
           (Hit_region.hit
              region
              ~transform
              ~clips:[ rect 85. 55. 10. 20.; rect 88. 60. 5. 10. ])
       : bool list)];
  print_s
    [%sexp
      (Rect.intersect (rect 0. 0. 10. 10.) (rect 10. 0. 10. 10.) |> Option.is_none : bool)];
  print_s
    [%sexp (Rect.intersect (rect 0. 0. 10. 10.) (rect 5. 6. 10. 10.) : Rect.t option)];
  print_s
    [%sexp
      (Hit_region.hit
         region
         ~transform:Transform.identity
         ~clips:[ rect 0. 0. 5. 5.; rect 5. 0. 5. 5. ]
         (point 5. 2.)
       : bool)];
  [%expect
    {|
    (true true true false false)
    (false false true false false)
    true
    (((x 5) (y 6) (width 5) (height 4)))
    false
    |}]
;;

let%expect_test "empty clip intersections and numerical edge tolerance" =
  let region = Hit_region.rectangle (rect 0. 0. 10. 10.) in
  let probes =
    [ point (-5e-8) 5.; point (-2e-7) 5.; point 10.00000005 5.; point 10.0000002 5. ]
  in
  print_s [%sexp (List.map probes ~f:(Hit_region.contains region) : bool list)];
  let polygon =
    Hit_region.polygon [ point 0. 0.; point 0.001 0.; point 0.001 0.001 ]
    |> Or_error.ok_exn
  in
  print_s
    [%sexp
      (Hit_region.contains polygon (point 0.0005 0.00050005) : bool)
    , (Hit_region.contains polygon (point 0.0005 0.0005002) : bool)];
  [%expect
    {|
    (true false true false)
    (true false)
    |}]
;;

let%expect_test "ellipse boundaries and even-odd concave polygon hits" =
  let ellipse = Hit_region.ellipse (rect 0. 0. 20. 10.) in
  print_s
    [%sexp
      (List.map
         [ point 10. 5.; point 20. 5.; point 0. 0.; point 20.0001 5. ]
         ~f:(Hit_region.contains ellipse)
       : bool list)];
  let points =
    [ point 0. 0.; point 10. 0.; point 10. 4.; point 4. 4.; point 4. 10.; point 0. 10. ]
  in
  let polygon = Hit_region.polygon points |> Or_error.ok_exn in
  let reversed = Hit_region.polygon (List.rev points) |> Or_error.ok_exn in
  let probes =
    [ point 1. 1.; point 8. 2.; point 2. 8.; point 8. 8.; point 4. 6.; point 4.001 6. ]
  in
  let hits = List.map probes ~f:(Hit_region.contains polygon) in
  print_s
    [%sexp
      (hits : bool list)
    , (List.equal Bool.equal hits (List.map probes ~f:(Hit_region.contains reversed))
       : bool)];
  print_s
    [%sexp
      (List.map
         [ []
         ; [ point 0. 0.; point 1. 1. ]
         ; [ point 0. 0.; point 1. 1.; point 2. 2. ]
         ; List.init 257 ~f:(fun i ->
             point (Float.of_int (i mod 2)) (Float.of_int (i mod 3)))
         ]
         ~f:(fun points -> Hit_region.polygon points |> Or_error.is_error)
       : bool list)];
  [%expect
    {|
    (true true false false)
    ((true true true false true false) true)
    (true true true true)
    |}]
;;

let%expect_test "affine inverse round trips across the admitted coordinate range" =
  let transforms =
    [ Transform.identity
    ; scale (-2.) 3.
    ; Transform.create ~a:1. ~b:0.5 ~c:0.25 ~d:1. ~tx:500. ~ty:(-300.) |> Or_error.ok_exn
    ; Transform.rotate ~radians:0.7 |> Or_error.ok_exn
    ]
  in
  let checks = ref 0 in
  List.iter transforms ~f:(fun transform ->
    for x = -10 to 10 do
      for y = -10 to 10 do
        let source = point (Float.of_int x *. 1234.) (Float.of_int y *. 789.) in
        let actual =
          Transform.apply transform source
          |> Or_error.bind ~f:(Transform.unapply transform)
          |> Or_error.ok_exn
        in
        assert (Float.(abs (Point.x actual -. Point.x source) < 1e-8));
        assert (Float.(abs (Point.y actual -. Point.y source) < 1e-8));
        incr checks
      done
    done);
  print_s [%sexp (!checks : int)];
  [%expect {| 1764 |}]
;;
