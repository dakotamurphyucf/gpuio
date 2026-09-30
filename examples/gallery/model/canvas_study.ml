open Core
module G = Gpuio.Canvas_geometry
module S = Gpuio.Canvas_scene
module R = Gpuio.Canvas_resource

let ok = Or_error.ok_exn
let point x y = G.Point.create ~x ~y |> ok
let rect x y width height = G.Rect.create ~x ~y ~width ~height |> ok
let id n = S.Item_id.of_int64 (Int64.of_int n) |> ok
let resource n = R.Id.of_int64 (Int64.of_int n) |> ok
let color = Gpuio.Color.rgb_exn
let paint rgb = S.Paint.create ~fill:(color rgb) () |> ok

module Item = struct
  type t =
    { id : S.Item_id.t
    ; name : string
    ; transform : G.Transform.t
    ; drawing : S.Drawing.t
    ; hit : G.Hit_region.t
    }

  let id t = t.id
  let name t = t.name
  let position t = G.Transform.apply t.transform (point 0. 0.) |> ok
end

type t =
  { items : Item.t list
  ; scene : S.t
  }

let build items =
  let open Or_error.Let_syntax in
  let grid =
    List.init 12 ~f:(fun column ->
      List.init 5 ~f:(fun row ->
        S.Item.create
          ~id:(id (1000 + (column * 5) + row))
          (S.Drawing.ellipse
             (rect
                (24. +. (Float.of_int column *. 48.))
                (24. +. (Float.of_int row *. 48.))
                2.
                2.)
             ~paint:(paint 0xc6d1de))
        |> ok))
    |> List.concat
  in
  let%bind drawings =
    List.mapi items ~f:(fun index item ->
      let%bind shape =
        S.Item.create
          ~id:item.Item.id
          ~transform:item.transform
          ~interaction:
            (S.Interaction.create
               ~label:item.name
               ~hit_region:item.hit
               ~draggable:true
               ~activatable:true
               ()
             |> ok)
          item.drawing
      in
      let%map label =
        S.Item.create
          ~id:(id (100 + index))
          ~transform:item.transform
          (S.Drawing.text
             (R.text ~id:(resource (100 + index)) ~font_size:14. item.name |> ok)
             ~origin:(point (-22.) 42.)
             ~color:(color 0x34445b))
      in
      [ shape; label ])
    |> Or_error.combine_errors
  in
  let%map scene =
    S.create
      ~description:"Three editable shapes on a paper canvas: Orbit, Prism and Tile."
      (grid @ List.concat drawings)
  in
  { items; scene }
;;

let initial =
  let bounds = rect (-30.) (-30.) 60. 60. in
  let diamond = [ point 0. (-36.); point 36. 0.; point 0. 36.; point (-36.) 0. ] in
  let path =
    Gpuio.Canvas_path.create
      [ Move (List.hd_exn diamond)
      ; Line (List.nth_exn diamond 1)
      ; Line (List.nth_exn diamond 2)
      ; Line (List.nth_exn diamond 3)
      ; Close
      ]
    |> ok
  in
  [ ( "Orbit"
    , 110.
    , 130.
    , S.Drawing.ellipse bounds ~paint:(paint 0x128774)
    , G.Hit_region.ellipse bounds )
  ; ( "Prism"
    , 290.
    , 100.
    , S.Drawing.path (R.path ~id:(resource 1) path |> ok) ~paint:(paint 0x8062cf) |> ok
    , G.Hit_region.polygon diamond |> ok )
  ; ( "Tile"
    , 470.
    , 160.
    , S.Drawing.rectangle bounds ~paint:(paint 0xd28b25)
    , G.Hit_region.rectangle bounds )
  ]
  |> List.mapi ~f:(fun index (name, x, y, drawing, hit) ->
    { Item.id = id (index + 1)
    ; name
    ; transform = G.Transform.translate ~x ~y |> ok
    ; drawing
    ; hit
    })
  |> build
  |> ok
;;

let items t = t.items
let scene t = t.scene
let find t wanted = List.find t.items ~f:(fun item -> S.Item_id.equal item.Item.id wanted)

let move t wanted transform =
  match find t wanted with
  | None -> Or_error.error_string "The canvas item is no longer present"
  | Some _ ->
    List.map t.items ~f:(fun item ->
      if S.Item_id.equal item.Item.id wanted then { item with transform } else item)
    |> build
;;
