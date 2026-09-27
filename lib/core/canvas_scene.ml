open Core
module Geometry = Canvas_geometry
module Resource = Canvas_resource
module Wire = Gpuio_protocol.Canvas_scene_wire
module G = Gpuio_protocol.Canvas_wire

module Item_id = struct
  type t = int64 [@@deriving equal, compare, sexp_of]

  let of_int64 t =
    if Int64.(t > 0L)
    then Ok t
    else Or_error.error_string "canvas item ID must be positive"
  ;;

  let to_int64 t = t
end

module Stroke = struct
  type t =
    { color : Color.t
    ; width : float
    }

  let create ~color ~width =
    if Float.is_finite width && Float.(width > 0. && width <= 256.)
    then Ok { color; width }
    else Or_error.error_string "canvas stroke width must be finite and in (0,256]"
  ;;

  let to_wire t theme =
    let%map.Or_error color = Theme.resolve theme t.color in
    { Wire.Stroke.color; width = t.width }
  ;;
end

module Paint = struct
  type t =
    { fill : Color.t option
    ; stroke : Stroke.t option
    }

  let create ?fill ?stroke () =
    if Option.is_none fill && Option.is_none stroke
    then Or_error.error_string "canvas paint needs a fill or stroke"
    else Ok { fill; stroke }
  ;;

  let to_wire t theme =
    let%bind.Or_error fill =
      match t.fill with
      | None -> Ok None
      | Some color -> Theme.resolve theme color |> Or_error.map ~f:Option.some
    in
    let%map.Or_error stroke =
      match t.stroke with
      | None -> Ok None
      | Some stroke -> Stroke.to_wire stroke theme |> Or_error.map ~f:Option.some
    in
    { Wire.Paint.fill; stroke }
  ;;
end

let rect_corners rect =
  let { G.Rect.x; y; width; height } = Geometry.Expert.rect_to_wire rect in
  List.map
    [ x, y; x +. width, y; x, y +. height; x +. width, y +. height ]
    ~f:(fun (x, y) -> { G.Point.x; y })
;;

module Drawing = struct
  type t =
    | Rectangle of Geometry.Rect.t * Paint.t
    | Ellipse of Geometry.Rect.t * Paint.t
    | Path of Resource.path Resource.t * Paint.t
    | Text of Resource.text Resource.t * Geometry.Point.t * Color.t
    | Image of Resource.image Resource.t * Geometry.Rect.t

  let rectangle rect ~paint = Rectangle (rect, paint)
  let ellipse rect ~paint = Ellipse (rect, paint)

  let path resource ~paint =
    if Option.is_some paint.Paint.fill && not (Resource.Expert.path_is_closed resource)
    then Or_error.error_string "filled canvas paths require explicitly closed contours"
    else Ok (Path (resource, paint))
  ;;

  let text resource ~origin ~color = Text (resource, origin, color)
  let image resource ~bounds = Image (resource, bounds)

  let resource = function
    | Rectangle _ | Ellipse _ -> None
    | Path (resource, _) -> Some (Resource.Expert.pack resource)
    | Text (resource, _, _) -> Some (Resource.Expert.pack resource)
    | Image (resource, _) -> Some (Resource.Expert.pack resource)
  ;;

  let corners = function
    | Rectangle (rect, _) | Ellipse (rect, _) | Image (_, rect) -> rect_corners rect
    | Path (resource, _) ->
      Resource.Expert.path_corners resource |> List.map ~f:Geometry.Expert.point_to_wire
    | Text (_, origin, _) -> [ Geometry.Expert.point_to_wire origin ]
  ;;

  let supports_transform t (transform : G.Transform.t) =
    let positive_axes =
      Float.(transform.a > 0. && transform.d > 0.)
      && Float.equal transform.b 0.
      && Float.equal transform.c 0.
    in
    match t with
    | Text _ -> positive_axes && Float.equal transform.a transform.d
    | Image _ -> positive_axes
    | Rectangle _ | Ellipse _ | Path _ -> true
  ;;

  let to_wire t theme =
    let key t = Resource.Expert.pack t |> Resource.Expert.key in
    match t with
    | Rectangle (rect, paint) ->
      let%map.Or_error paint = Paint.to_wire paint theme in
      Wire.Drawing.Shape (Rectangle (Geometry.Expert.rect_to_wire rect), paint)
    | Ellipse (rect, paint) ->
      let%map.Or_error paint = Paint.to_wire paint theme in
      Wire.Drawing.Shape (Ellipse (Geometry.Expert.rect_to_wire rect), paint)
    | Path (resource, paint) ->
      let%map.Or_error paint = Paint.to_wire paint theme in
      Wire.Drawing.Shape (Path (key resource), paint)
    | Text (resource, origin, color) ->
      let%map.Or_error color = Theme.resolve theme color in
      Wire.Drawing.Text (key resource, Geometry.Expert.point_to_wire origin, color)
    | Image (resource, rect) ->
      Ok (Wire.Drawing.Image (key resource, Geometry.Expert.rect_to_wire rect))
  ;;
end

module Interaction = struct
  type t = Wire.Interaction.t

  let create ~label ~hit_region ?(draggable = false) ?(activatable = false) () =
    if
      String.is_empty (String.strip label)
      || String.length label > 1024
      || (not (Stdlib.String.is_valid_utf_8 label))
      || String.contains label '\000'
    then
      Or_error.error_string
        "canvas interaction label must be nonblank UTF-8 without NUL, <=1024 bytes"
    else
      Ok
        { Wire.Interaction.label
        ; hit_region = Geometry.Expert.hit_region_to_wire hit_region
        ; draggable
        ; activatable
        }
  ;;

  let corners t =
    match t.Wire.Interaction.hit_region with
    | Rectangle rect | Ellipse rect ->
      let { G.Rect.x; y; width; height } = rect in
      List.map
        [ x, y; x +. width, y; x, y +. height; x +. width, y +. height ]
        ~f:(fun (x, y) -> { G.Point.x; y })
    | Polygon points -> points
  ;;
end

module Item = struct
  type t =
    { id : Item_id.t
    ; transform : G.Transform.t
    ; clips : G.Rect.t list
    ; drawing : Drawing.t
    ; interaction : Interaction.t option
    }

  let create
        ~id
        ?(transform = Geometry.Transform.identity)
        ?(clips = [])
        ?interaction
        drawing
    =
    let transform = Geometry.Expert.transform_to_wire transform in
    let corners =
      Drawing.corners drawing
      @ Option.value_map interaction ~default:[] ~f:Interaction.corners
    in
    if List.length clips > Wire.max_clips
    then Or_error.error_string "canvas item exceeds eight clips"
    else if not (Drawing.supports_transform drawing transform)
    then Or_error.error_string "unsupported text/image canvas transform"
    else if
      not
        (List.for_all corners ~f:(fun p -> G.Transform.apply transform p |> G.Point.valid))
    then Or_error.error_string "transformed canvas geometry exceeds the coordinate domain"
    else
      Ok
        { id
        ; transform
        ; clips = List.map clips ~f:Geometry.Expert.rect_to_wire
        ; drawing
        ; interaction
        }
  ;;

  let id t = t.id

  let to_wire t theme =
    let%map.Or_error drawing = Drawing.to_wire t.drawing theme in
    { Wire.Item.id = t.id
    ; transform = t.transform
    ; clips = t.clips
    ; drawing
    ; interaction = t.interaction
    }
  ;;
end

type t =
  { wire : Wire.t
  ; resources : Resource.Expert.packed list
  ; encoded_bytes : int
  ; retained_bytes : int
  }

let equal = phys_equal
let item_count t = List.length t.wire.items
let resource_count t = List.length t.resources
let encoded_bytes t = t.encoded_bytes

let collect_resources items =
  List.fold_result items ~init:Int64.Map.empty ~f:(fun resources item ->
    match Drawing.resource item.Item.drawing with
    | None -> Ok resources
    | Some resource ->
      let key = Resource.Expert.key resource in
      (match Map.find resources key.id with
       | Some previous when Resource.Expert.equal_packed previous resource -> Ok resources
       | Some _ -> Or_error.error_string "conflicting canvas resource identity"
       | None ->
         if Map.length resources >= Wire.max_resources
         then Or_error.error_string "canvas exceeds 4096 resources"
         else Ok (Map.add_exn resources ~key:key.id ~data:resource)))
  |> Or_error.map ~f:Map.data
;;

let create ~description ?(theme = Theme.default) items =
  let open Or_error.Let_syntax in
  if
    String.is_empty (String.strip description)
    || String.length description > 4096
    || (not (Stdlib.String.is_valid_utf_8 description))
    || String.contains description '\000'
  then
    Or_error.error_string
      "canvas description must be nonblank UTF-8 without NUL, <=4096 bytes"
  else if List.length items > Wire.max_items
  then Or_error.error_string "canvas exceeds 20000 items"
  else (
    let ids = List.map items ~f:Item.id in
    if Set.length (Int64.Set.of_list ids) <> List.length ids
    then Or_error.error_string "duplicate canvas item ID"
    else (
      let%bind resources = collect_resources items in
      let path_commands =
        List.sum (module Int) resources ~f:Resource.Expert.path_commands
      in
      let interactive = List.filter_map items ~f:(fun item -> item.Item.interaction) in
      let text_bytes =
        String.length description
        + List.sum (module Int) resources ~f:Resource.Expert.text_bytes
        + List.sum
            (module Int)
            interactive
            ~f:(fun i -> String.length i.Wire.Interaction.label)
      in
      if
        path_commands > Wire.max_path_commands
        || text_bytes > Wire.max_text_bytes
        || List.length interactive > Wire.max_interactive_items
      then
        Or_error.error_string "canvas aggregate drawing/text/interaction budget exceeded"
      else (
        let%bind wire_items =
          List.map items ~f:(fun item -> Item.to_wire item theme) |> Or_error.all
        in
        let wire =
          { Wire.version = 1L
          ; description
          ; resources = List.map resources ~f:Resource.Expert.to_wire
          ; items = wire_items
          }
        in
        let encoded_bytes = Wire.bin_size_t wire in
        if encoded_bytes > Wire.max_bytes
        then Or_error.error_string "canvas encoded scene exceeds 4 MiB"
        else (
          (* Conservative logical charge for boxed geometry, list cells, records,
             resource canonical strings and one encoded upload buffer. Count
             shared resources once per scene; cross-scene sharing is deliberately
             overcharged. This bounds adapter-owned retention, not GC/RSS. *)
          let geometry =
            List.sum
              (module Int)
              wire_items
              ~f:(fun item ->
                (128 * List.length item.clips)
                + Option.value_map item.interaction ~default:0 ~f:(fun interaction ->
                  match interaction.hit_region with
                  | Rectangle _ | Ellipse _ -> 128
                  | Polygon points -> 128 * List.length points))
          in
          let retained_bytes =
            4096
            + (4 * encoded_bytes)
            + (512 * List.length wire_items)
            + (512 * List.length resources)
            + (256 * path_commands)
            + geometry
          in
          Ok { wire; resources; encoded_bytes; retained_bytes }))))
;;

module Owner = struct
  type t = unit ref

  let create () = ref ()
  let equal = phys_equal
  let sexp_of_t _ = Sexp.Atom "<application>"
end

module Handle = struct
  type t =
    { owner : Owner.t
    ; id : Gpuio_protocol.Resource_id.t
    }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  module Owner = Owner

  let retained_bytes t = t.retained_bytes
  let handle ~owner id = { Handle.owner; id }
  let belongs_to (handle : Handle.t) ~owner = Owner.equal handle.owner owner
  let native_id (handle : Handle.t) = handle.id

  let assets_belong_to t ~asset_owner =
    List.for_all t.resources ~f:(fun resource ->
      Resource.Expert.belongs_to resource ~asset_owner)
  ;;

  let encode t ~asset_owner =
    if not (assets_belong_to t ~asset_owner)
    then Or_error.error_string "canvas image belongs to another application"
    else Ok (Bin_prot.Utils.bin_dump Wire.bin_writer_t t.wire |> Bigstring.to_string)
  ;;
end
