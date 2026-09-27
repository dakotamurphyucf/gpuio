open Core
module G = Gpuio.Canvas_geometry
module S = Gpuio.Canvas_scene
module R = Gpuio.Canvas_resource

let ok = Or_error.ok_exn
let point x y = G.Point.create ~x ~y |> ok
let id value = S.Item_id.of_int64 value |> ok
let resource_id value = R.Id.of_int64 value |> ok

module Stage = struct
  type t =
    | Read
    | Draft
    | Review
  [@@deriving equal, sexp_of]

  let all = [ Read; Draft; Review ]

  let name = function
    | Read -> "Read sources"
    | Draft -> "Draft changes"
    | Review -> "Review output"
  ;;

  let description = function
    | Read -> "Gather the selected workspace sources before proposing a change."
    | Draft ->
      "Prepare a patch from the source context, keeping the original files intact."
    | Review -> "Inspect the proposed output and record a review checkpoint."
  ;;

  let next = function
    | Read -> Draft
    | Draft -> Review
    | Review -> Read
  ;;

  let id = function
    | Read -> id 1L
    | Draft -> id 2L
    | Review -> id 3L
  ;;

  let of_id value = List.find all ~f:(fun stage -> S.Item_id.equal (id stage) value)
end

type t = (Stage.t * G.Transform.t) list

let create () =
  List.mapi Stage.all ~f:(fun index stage ->
    stage, G.Transform.translate ~x:34. ~y:(42. +. (Float.of_int index *. 94.)) |> ok)
;;

let transform t stage = List.Assoc.find_exn t stage ~equal:Stage.equal
let position t stage = G.Transform.apply (transform t stage) (point 0. 0.) |> ok

let move t stage transform =
  let open Or_error.Let_syntax in
  let%bind position = G.Transform.apply transform (point 0. 0.) in
  let x = G.Point.x position
  and y = G.Point.y position in
  let%bind translation = G.Transform.translate ~x ~y in
  if not (G.Transform.equal translation transform)
  then Or_error.error_string "Stage labels support translation only"
  else if Float.(abs x > 999_000. || abs y > 999_000.)
  then Or_error.error_string "Stage position leaves no room for its connectors"
  else Ok (List.Assoc.add t ~equal:Stage.equal stage transform)
;;

let scene
      ?(annotation = Gpuio.Color_value.Value.Empty)
      t
      ~(palette : Palette.t)
      ~generation
  =
  let annotation =
    match annotation with
    | Empty -> palette.accent
    | Color color -> Gpuio.Color_value.Rgba.to_color color
  in
  let nodes =
    List.mapi Stage.all ~f:(fun index stage ->
      let label = sprintf "%02d  %s" (index + 1) (Stage.name stage) in
      let resource =
        R.text
          ~id:(resource_id (Int64.of_int (index + 1)))
          ~font_size:16.
          ~font_weight:600
          label
        |> ok
      in
      let bounds = G.Rect.create ~x:(-10.) ~y:(-8.) ~width:260. ~height:40. |> ok in
      let interaction =
        S.Interaction.create
          ~label:(Stage.name stage)
          ~hit_region:(G.Hit_region.rectangle bounds)
          ~draggable:true
          ~activatable:true
          ()
        |> ok
      in
      S.Item.create
        ~id:(Stage.id stage)
        ~transform:(transform t stage)
        ~interaction
        (S.Drawing.text resource ~origin:(point 0. 0.) ~color:palette.text)
      |> ok)
  in
  let links =
    List.mapi
      [ Stage.Read, Stage.Draft; Stage.Draft, Stage.Review ]
      ~f:(fun index (src, dst) ->
        let src = position t src
        and dst = position t dst in
        let x = G.Point.x dst +. 12.
        and y = G.Point.y dst -. 16. in
        let path =
          Gpuio.Canvas_path.create
            [ Move (point (G.Point.x src +. 12.) (G.Point.y src +. 33.))
            ; Line (point x y)
            ; Move (point (x -. 4.) (y -. 6.))
            ; Line (point x y)
            ; Line (point (x +. 4.) (y -. 6.))
            ]
          |> ok
        in
        let number = Int64.of_int (10 + index) in
        let resource = R.path ~id:(resource_id number) ~generation path |> ok in
        let stroke = S.Stroke.create ~color:annotation ~width:1.5 |> ok in
        let paint = S.Paint.create ~stroke () |> ok in
        S.Item.create ~id:(id number) (S.Drawing.path resource ~paint |> ok) |> ok)
  in
  S.create
    ~description:
      "Simulated run: Read sources → Draft changes → Review output. Select or move a \
       stage; no real files are changed. The stage list below provides the same \
       information without the diagram."
    (links @ nodes)
  |> ok
;;
