open Core
module Canvas = Gpuio.Canvas
module Scene = Gpuio.Canvas_scene
module G = Gpuio.Canvas_geometry
module Wire = Gpuio_protocol.Canvas_view_wire

let ok = Or_error.ok_exn
let point x y = G.Point.create ~x ~y |> ok
let viewport x y zoom = Canvas.Viewport.create ~origin:(point x y) ~zoom |> ok
let native_id = Gpuio_protocol.Resource_id.create ~slot:7L ~generation:2L |> ok
let owner = Scene.Expert.Owner.create ()
let handle = Scene.Expert.handle ~owner native_id

let%expect_test
    "view configuration preserves owner until encoding and matches native fixture"
  =
  let command =
    Canvas.Command.create ~sequence:9L (Set_viewport (viewport 5. (-3.) 2.75)) |> ok
  in
  let config =
    Canvas.Config.create
      ~scene:handle
      ~label:"Canvas 🦀"
      ~initial_viewport:(viewport (-2.5) 4.25 1.5)
      ~minimum_zoom:0.25
      ~maximum_zoom:8.
      ~pan_zoom:false
      ~selection_color:(Gpuio.Color.token_exn "selection")
      ~theme:(Gpuio.Theme.create [ "selection", Gpuio.Color.rgb_exn 0x102030 ] |> ok)
      ~command
      ()
    |> ok
  in
  let wire = Canvas.Expert.to_wire config ~owner:(Some owner) in
  assert (Wire.Config.valid wire);
  assert (Option.equal Gpuio_protocol.Resource_id.equal wire.source (Some native_id));
  assert (Option.is_none (Canvas.Expert.to_wire config ~owner:None).source);
  assert (
    Option.is_none
      (Canvas.Expert.to_wire config ~owner:(Some (Scene.Expert.Owner.create ()))).source);
  let bytes =
    Bin_prot.Utils.bin_dump Wire.Config.bin_writer_t wire |> Bigstring.to_string
  in
  let hex =
    String.to_list bytes
    |> List.map ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
    |> String.concat
  in
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "canvas-v1-view.hex") |> String.strip
    in
    assert (String.equal expected hex));
  print_s [%sexp (String.length bytes : int)];
  [%expect {| 91 |}]
;;

let%expect_test
    "invalid bounds, labels, unknown colors and command sequences are rejected"
  =
  List.iter [ Float.nan; Float.infinity; 0.; 0.049; 64.01 ] ~f:(fun zoom ->
    assert (Result.is_error (Canvas.Viewport.create ~origin:(point 0. 0.) ~zoom)));
  List.iter
    [ ""; " \011\t\r\n"; "\000"; "\255"; String.make 1025 'x' ]
    ~f:(fun label ->
      assert (Result.is_error (Canvas.Config.create ~scene:handle ~label ())));
  List.iter
    [ 0., 1.; 2., 1.; 0.05, 65.; Float.nan, 2.; 0.05, Float.infinity ]
    ~f:(fun (minimum_zoom, maximum_zoom) ->
      assert (
        Result.is_error
          (Canvas.Config.create ~scene:handle ~minimum_zoom ~maximum_zoom ())));
  assert (
    Result.is_error
      (Canvas.Config.create
         ~scene:handle
         ~selection_color:(Gpuio.Color.token_exn "missing")
         ()));
  assert (Result.is_error (Canvas.Command.create ~sequence:0L Reset_positions));
  assert (Result.is_error (Canvas.Command.create ~sequence:(-1L) Reset_viewport));
  assert (
    Result.is_ok (Canvas.Config.create ~scene:handle ~minimum_zoom:1. ~maximum_zoom:1. ()));
  print_endline "invalid configuration rejected";
  [%expect {| invalid configuration rejected |}]
;;

let%expect_test
    "observations validate item IDs, transforms, viewport and publication identity"
  =
  let events : Wire.Observation.t list =
    [ Selection_changed None
    ; Selection_changed (Some 1L)
    ; Activated 2L
    ; Moved (3L, Gpuio_protocol.Canvas_wire.Transform.identity)
    ; Viewport_changed { origin = { x = 5.; y = 6. }; zoom = 2. }
    ; Command_completed 9L
    ; Failed Render_limit
    ]
  in
  List.iter events ~f:(fun event ->
    assert (
      Result.is_ok (Canvas.Expert.event ~scene_revision:2L ~scene_generation:3L event)));
  List.iter
    [ Wire.Observation.Selection_changed (Some 0L)
    ; Activated (-1L)
    ; Moved (1L, { Gpuio_protocol.Canvas_wire.Transform.identity with a = 0. })
    ; Viewport_changed { origin = { x = Float.nan; y = 0. }; zoom = 1. }
    ; Command_completed 0L
    ]
    ~f:(fun event ->
      assert (
        Result.is_error
          (Canvas.Expert.event ~scene_revision:2L ~scene_generation:3L event)));
  assert (
    Result.is_error
      (Canvas.Expert.event ~scene_revision:0L ~scene_generation:0L (Activated 1L)));
  assert (
    Result.is_error
      (Canvas.Expert.event
         ~scene_revision:0L
         ~scene_generation:1L
         (Failed Unavailable_scene)));
  assert (
    Result.is_ok
      (Canvas.Expert.event
         ~scene_revision:0L
         ~scene_generation:0L
         (Failed Wrong_application)));
  print_endline "bounded semantic observations";
  [%expect {| bounded semantic observations |}]
;;
