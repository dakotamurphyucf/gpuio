open Core
module Scene = Gpuio.Canvas_scene
module Resource = Gpuio.Canvas_resource
module Geometry = Gpuio.Canvas_geometry
module Path = Gpuio.Canvas_path
module Asset = Gpuio.Asset
module Color = Gpuio.Color

let ok = Or_error.ok_exn
let resource_id id = Resource.Id.of_int64 (Int64.of_int id) |> ok
let item_id id = Scene.Item_id.of_int64 (Int64.of_int id) |> ok
let point x y = Geometry.Point.create ~x ~y |> ok
let rect = Geometry.Rect.create ~x:0. ~y:0. ~width:20. ~height:10. |> ok
let fill = Scene.Paint.create ~fill:(Color.rgb_exn 0x102030) () |> ok

let item ?transform ?clips ?interaction id drawing =
  Scene.Item.create ~id:(item_id id) ?transform ?clips ?interaction drawing |> ok
;;

let rectangle id = item id (Scene.Drawing.rectangle rect ~paint:fill)

let asset owner =
  Asset.Expert.handle
    ~owner
    ~id:(Gpuio_protocol.Resource_id.create ~slot:7L ~generation:2L |> ok)
    ~format:Png
;;

let%expect_test "public scene constructors produce the independent native fixture" =
  let owner = Asset.Expert.Owner.create () in
  let stroke = Scene.Stroke.create ~color:(Color.rgb_exn 0xff0000) ~width:2. |> ok in
  let paint = Scene.Paint.create ~fill:(Color.token_exn "accent") ~stroke () |> ok in
  let path =
    Path.create [ Move (point 0. 0.); Line (point 10. 10.); Line (point 0. 10.); Close ]
    |> ok
    |> Resource.path ~id:(resource_id 1) ~generation:2L
    |> ok
  in
  let text = Resource.text ~id:(resource_id 2) ~font_weight:500 "héllo 🦀" |> ok in
  let image = Resource.image ~id:(resource_id 3) ~generation:3L (asset owner) |> ok in
  let interact label hit_region ~draggable ~activatable =
    Scene.Interaction.create ~label ~hit_region ~draggable ~activatable () |> ok
  in
  let items =
    [ item
        1
        ~clips:[ rect ]
        ~interaction:
          (interact
             "Rectangle"
             (Geometry.Hit_region.rectangle rect)
             ~draggable:true
             ~activatable:true)
        (Scene.Drawing.rectangle rect ~paint)
    ; item
        2
        ~transform:(Geometry.Transform.translate ~x:30. ~y:0. |> ok)
        ~interaction:
          (interact
             "Ellipse"
             (Geometry.Hit_region.ellipse rect)
             ~draggable:false
             ~activatable:true)
        (Scene.Drawing.ellipse
           rect
           ~paint:(Scene.Paint.create ~fill:(Color.rgb_exn 0xff00ff) () |> ok))
    ; item
        3
        ~transform:(Geometry.Transform.translate ~x:0. ~y:30. |> ok)
        ~interaction:
          (interact
             "Path"
             (Geometry.Hit_region.polygon [ point 0. 0.; point 10. 10.; point 0. 10. ]
              |> ok)
             ~draggable:true
             ~activatable:false)
        (Scene.Drawing.path path ~paint:(Scene.Paint.create ~stroke () |> ok) |> ok)
    ; item
        4
        (Scene.Drawing.text text ~origin:(point 10. 50.) ~color:(Color.rgb_exn 0xffffff))
    ; item 5 (Scene.Drawing.image image ~bounds:rect)
    ]
  in
  let theme = Gpuio.Theme.create [ "accent", Color.rgb_exn 0x102030 ] |> ok in
  let scene = Scene.create ~description:"Diagram 🦀" ~theme items |> ok in
  let bytes = Scene.Expert.encode scene ~asset_owner:owner |> ok in
  assert (String.length bytes = Scene.encoded_bytes scene);
  Eio_main.run (fun env ->
    let expected =
      Eio.Path.load Eio.Path.(Eio.Stdenv.cwd env / "canvas-v1-scene.hex") |> String.strip
    in
    let actual =
      String.to_list bytes
      |> List.map ~f:(fun byte -> sprintf "%02x" (Char.to_int byte))
      |> String.concat
    in
    assert (String.equal expected actual));
  print_s [%sexp (Scene.item_count scene : int), (Scene.resource_count scene : int)];
  [%expect {| (5 3) |}]
;;

let%expect_test
    "identity, resources, text, paint and transformed geometry validate locally"
  =
  let text = Resource.text ~id:(resource_id 1) "Label" |> ok in
  let open_path =
    Path.create [ Move (point 0. 0.); Line (point 10. 10.) ]
    |> ok
    |> Resource.path ~id:(resource_id 2)
    |> ok
  in
  let invalid =
    [ Resource.Id.of_int64 0L |> Or_error.is_error
    ; Scene.Item_id.of_int64 (-1L) |> Or_error.is_error
    ; Resource.text ~id:(resource_id 1) ~generation:0L "text" |> Or_error.is_error
    ; Resource.text ~id:(resource_id 1) "a\nb" |> Or_error.is_error
    ; Resource.text ~id:(resource_id 1) "\255" |> Or_error.is_error
    ; Resource.text ~id:(resource_id 1) ~font_size:Float.nan "text" |> Or_error.is_error
    ; Resource.text ~id:(resource_id 1) ~font_family:"\011" "text" |> Or_error.is_error
    ; Scene.Paint.create () |> Or_error.is_error
    ; Scene.Stroke.create ~color:(Color.rgb_exn 0) ~width:0. |> Or_error.is_error
    ; Scene.Drawing.path open_path ~paint:fill |> Or_error.is_error
    ; Scene.Item.create
        ~id:(item_id 1)
        ~transform:(Geometry.Transform.rotate ~radians:0.5 |> ok)
        (Scene.Drawing.text text ~origin:(point 0. 0.) ~color:(Color.rgb_exn 0))
      |> Or_error.is_error
    ; Scene.Item.create
        ~id:(item_id 1)
        ~transform:(Geometry.Transform.translate ~x:1_000_000. ~y:0. |> ok)
        (Scene.Drawing.rectangle rect ~paint:fill)
      |> Or_error.is_error
    ; Scene.Item.create
        ~id:(item_id 1)
        ~clips:(List.init 9 ~f:(fun _ -> rect))
        (Scene.Drawing.rectangle rect ~paint:fill)
      |> Or_error.is_error
    ; Scene.create ~description:"scene" [ rectangle 1; rectangle 1 ] |> Or_error.is_error
    ; Scene.create ~description:"\011" [] |> Or_error.is_error
    ]
  in
  print_s [%sexp (invalid : bool list)];
  [%expect
    {| (true true true true true true true true true true true true true true true) |}]
;;

let%expect_test "resource conflicts and image ownership cannot disappear during encoding" =
  let drawing resource =
    Scene.Drawing.text resource ~origin:(point 0. 0.) ~color:(Color.rgb_exn 0)
  in
  let first = Resource.text ~id:(resource_id 1) "one" |> ok in
  let changed = Resource.text ~id:(resource_id 1) "two" |> ok in
  let next_generation = Resource.text ~id:(resource_id 1) ~generation:2L "one" |> ok in
  print_s
    [%sexp
      (List.map [ changed; next_generation ] ~f:(fun second ->
         Scene.create
           ~description:"scene"
           [ item 1 (drawing first); item 2 (drawing second) ]
         |> Or_error.is_error)
       : bool list)];
  let shared =
    Scene.create ~description:"scene" [ item 1 (drawing first); item 2 (drawing first) ]
    |> ok
  in
  print_s [%sexp (Scene.resource_count shared : int)];
  let owner = Asset.Expert.Owner.create ()
  and foreign = Asset.Expert.Owner.create () in
  let image = Resource.image ~id:(resource_id 1) (asset owner) |> ok in
  let scene =
    Scene.create ~description:"image" [ item 1 (Scene.Drawing.image image ~bounds:rect) ]
    |> ok
  in
  print_s
    [%sexp
      (Scene.Expert.assets_belong_to scene ~asset_owner:owner : bool)
    , (Scene.Expert.encode scene ~asset_owner:foreign |> Or_error.is_error : bool)];
  let unknown = Scene.Paint.create ~fill:(Color.token_exn "missing") () |> ok in
  print_s
    [%sexp
      (Scene.create
         ~description:"scene"
         [ item 1 (Scene.Drawing.rectangle rect ~paint:unknown) ]
       |> Or_error.is_error
       : bool)];
  print_s
    [%sexp
      (Scene.equal shared shared : bool)
    , (Scene.equal
         shared
         (Scene.create
            ~description:"scene"
            [ item 1 (drawing first); item 2 (drawing first) ]
          |> ok)
       : bool)];
  [%expect
    {|
    (true true)
    1
    (true true)
    true
    (true false)
    |}]
;;

let%expect_test
    "large shared-resource scenes count geometry once and enforce aggregate limits"
  =
  let commands =
    Path.Command.Move (point 0. 0.)
    :: List.init 4095 ~f:(fun _ -> Path.Command.Line (point 10. 10.))
  in
  let path = Path.create commands |> ok |> Resource.path ~id:(resource_id 1) |> ok in
  let stroke = Scene.Stroke.create ~color:(Color.rgb_exn 0) ~width:1. |> ok in
  let drawing =
    Scene.Drawing.path path ~paint:(Scene.Paint.create ~stroke () |> ok) |> ok
  in
  let items = List.init 20_000 ~f:(fun i -> item (i + 1) drawing) in
  let scene = Scene.create ~description:"large" items |> ok in
  print_s
    [%sexp
      (Scene.item_count scene : int)
    , (Scene.resource_count scene : int)
    , (Scene.encoded_bytes scene < 4 * 1024 * 1024 : bool)];
  print_s
    [%sexp
      (Scene.create ~description:"large" (item 20_001 drawing :: items)
       |> Or_error.is_error
       : bool)];
  let clipped =
    List.init 20_000 ~f:(fun i ->
      item
        ~clips:(List.init 8 ~f:(fun _ -> rect))
        (i + 1)
        (Scene.Drawing.rectangle rect ~paint:fill))
  in
  print_s
    [%sexp (Scene.create ~description:"oversized" clipped |> Or_error.is_error : bool)];
  let resources =
    List.init 17 ~f:(fun i ->
      let resource =
        Resource.path ~id:(resource_id (i + 1)) (Path.create commands |> ok) |> ok
      in
      item
        (i + 1)
        (Scene.Drawing.path resource ~paint:(Scene.Paint.create ~stroke () |> ok) |> ok))
  in
  print_s
    [%sexp
      (Scene.create ~description:"too many commands" resources |> Or_error.is_error
       : bool)];
  [%expect
    {|
    (20000 1 true)
    true
    true
    true
    |}]
;;

let%expect_test "handle owners and canonical float encodings preserve exact identities" =
  let owner = Scene.Expert.Owner.create ()
  and other = Scene.Expert.Owner.create () in
  let id = Gpuio_protocol.Resource_id.create ~slot:0L ~generation:1L |> ok in
  let handle = Scene.Expert.handle ~owner id in
  print_s
    [%sexp
      (Scene.Expert.belongs_to handle ~owner : bool)
    , (Scene.Expert.belongs_to handle ~owner:other : bool)
    , (Scene.Handle.equal handle (Scene.Expert.handle ~owner:other id) : bool)];
  let make x =
    Path.create [ Move (point x 0.); Line (point 10. 10.) ]
    |> ok
    |> Resource.path ~id:(resource_id 1)
    |> ok
  in
  let positive = make 0.
  and negative = make (-0.) in
  print_s [%sexp (Resource.equal positive negative : bool)];
  let stroke = Scene.Stroke.create ~color:(Color.rgb_exn 0) ~width:1. |> ok in
  let paint = Scene.Paint.create ~stroke () |> ok in
  let drawing resource = Scene.Drawing.path resource ~paint |> ok in
  print_s
    [%sexp
      (Scene.create
         ~description:"canonical identity"
         [ item 1 (drawing positive); item 2 (drawing negative) ]
       |> Or_error.is_error
       : bool)];
  [%expect
    {|
    (true false false)
    false
    true
    |}]
;;

let%expect_test "resource, text and accessibility budgets are independent" =
  let text_item id value =
    let text = Resource.text ~id:(resource_id id) value |> ok in
    item id (Scene.Drawing.text text ~origin:(point 0. 0.) ~color:(Color.rgb_exn 0))
  in
  let errors =
    [ Scene.create
        ~description:"resources"
        (List.init 4097 ~f:(fun index -> text_item (index + 1) "x"))
      |> Or_error.is_error
    ; Scene.create
        ~description:"text"
        (List.init 65 ~f:(fun index -> text_item (index + 1) (String.make 16_384 'x')))
      |> Or_error.is_error
    ; (let interaction =
         Scene.Interaction.create
           ~label:"Item"
           ~hit_region:(Geometry.Hit_region.rectangle rect)
           ()
         |> ok
       in
       Scene.create
         ~description:"interactive"
         (List.init 2049 ~f:(fun index ->
            item (index + 1) ~interaction (Scene.Drawing.rectangle rect ~paint:fill)))
       |> Or_error.is_error)
    ]
  in
  print_s [%sexp (errors : bool list)];
  let owner = Asset.Expert.Owner.create () in
  let image = Resource.image ~id:(resource_id 1) (asset owner) |> ok in
  let drawing = Scene.Drawing.image image ~bounds:rect in
  print_s
    [%sexp
      (Scene.Item.create
         ~id:(item_id 1)
         ~transform:(Geometry.Transform.scale ~x:2. ~y:3. |> ok)
         drawing
       |> Or_error.is_ok
       : bool)
    , (Scene.Item.create
         ~id:(item_id 1)
         ~transform:(Geometry.Transform.scale ~x:(-1.) ~y:1. |> ok)
         drawing
       |> Or_error.is_error
       : bool)];
  [%expect
    {|
    (true true true)
    (true true)
    |}]
;;
