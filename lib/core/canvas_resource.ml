open Core
module Wire = Gpuio_protocol.Canvas_scene_wire
module Geometry = Gpuio_protocol.Canvas_wire

module Id = struct
  type t = int64 [@@deriving equal, compare, sexp_of]

  let of_int64 t =
    if Int64.(t > 0L)
    then Ok t
    else Or_error.error_string "canvas resource ID must be positive"
  ;;

  let to_int64 t = t
end

type path = unit
type text = unit
type image = unit

type data =
  | Path of Canvas_path.t
  | Text of Wire.Text.t
  | Image of Asset.Handle.t

type resource =
  { id : Id.t
  ; generation : int64
  ; data : data
  ; closed : bool
  ; corners : Canvas_geometry.Point.t list
  ; canonical : string
  }

type 'kind t = resource

let id t = t.id
let generation t = t.generation

let equal t other =
  phys_equal t other
  || (Id.equal t.id other.id
      && Int64.equal t.generation other.generation
      && String.equal t.canonical other.canonical
      &&
      match t.data, other.data with
      | Image first, Image second -> Asset.Handle.equal first second
      | Path _, Path _ | Text _, Text _ -> true
      | (Path _ | Text _ | Image _), _ -> false)
;;

let sexp_of_t t =
  let kind =
    match t.data with
    | Path _ -> "path"
    | Text _ -> "text"
    | Image _ -> "image"
  in
  [%sexp { id = (t.id : Id.t); generation = (t.generation : int64); kind : string }]
;;

let check_generation generation =
  if Int64.(generation > 0L)
  then Ok ()
  else Or_error.error_string "canvas resource generation must be positive"
;;

let wire_data = function
  | Path path -> Wire.Resource.Data.Path (Canvas_path.Expert.to_wire path)
  | Text text -> Text text
  | Image asset -> Image (Asset.Expert.native_id asset)
;;

let make ~id ~generation ~data ~closed ~corners =
  let canonical =
    Bin_prot.Utils.bin_dump Wire.Resource.Data.bin_writer_t (wire_data data)
    |> Bigstring.to_string
  in
  { id; generation; data; closed; corners; canonical }
;;

let path_corners path =
  let bounds =
    ref (Float.infinity, Float.infinity, Float.neg_infinity, Float.neg_infinity)
  in
  let extend (p : Geometry.Point.t) =
    let left, top, right, bottom = !bounds in
    bounds
    := Float.min left p.x, Float.min top p.y, Float.max right p.x, Float.max bottom p.y
  in
  List.iter (Canvas_path.Expert.to_wire path) ~f:(function
    | Move p | Line p -> extend p
    | Quadratic (control, p) ->
      extend control;
      extend p
    | Cubic (first, second, p) ->
      extend first;
      extend second;
      extend p
    | Close -> ());
  let left, top, right, bottom = !bounds in
  List.map
    [ left, top; right, top; left, bottom; right, bottom ]
    ~f:(fun (x, y) -> Canvas_geometry.Point.create ~x ~y |> Or_error.ok_exn)
;;

let path ~id ?(generation = 1L) path =
  let%map.Or_error () = check_generation generation in
  make
    ~id
    ~generation
    ~data:(Path path)
    ~closed:(Canvas_path.is_closed path)
    ~corners:(path_corners path)
;;

let valid_text value ~max_bytes =
  String.length value <= max_bytes
  && Stdlib.String.is_valid_utf_8 value
  && not (String.contains value '\000')
;;

let text
      ~id
      ?(generation = 1L)
      ?(font_family = "system")
      ?(font_size = 14.)
      ?(font_weight = 400)
      value
  =
  let%bind.Or_error () = check_generation generation in
  if
    String.is_empty value
    || (not (valid_text value ~max_bytes:16_384))
    || String.exists value ~f:(function
      | '\r' | '\n' -> true
      | _ -> false)
    || String.is_empty (String.strip font_family)
    || (not (valid_text font_family ~max_bytes:128))
    || (not (Float.is_finite font_size && Float.(font_size >= 4. && font_size <= 256.)))
    || font_weight < 100
    || font_weight > 900
  then Or_error.error_string "invalid canvas text or font specification"
  else
    Ok
      (make
         ~id
         ~generation
         ~data:
           (Text { value; font_family; font_size; font_weight = Int64.of_int font_weight })
         ~closed:false
         ~corners:[])
;;

let image ~id ?(generation = 1L) asset =
  let%map.Or_error () = check_generation generation in
  make ~id ~generation ~data:(Image asset) ~closed:false ~corners:[]
;;

module Expert = struct
  type packed = resource

  let pack t = t
  let equal_packed = equal
  let key t : Wire.Resource_key.t = { id = t.id; generation = t.generation }
  let to_wire t : Wire.Resource.t = { key = key t; data = wire_data t.data }

  let belongs_to t ~asset_owner =
    match t.data with
    | Path _ | Text _ -> true
    | Image asset -> Asset.Expert.belongs_to asset ~owner:asset_owner
  ;;

  let path_is_closed t = t.closed
  let path_corners t = t.corners

  let path_commands t =
    match t.data with
    | Path path -> Canvas_path.command_count path
    | Text _ | Image _ -> 0
  ;;

  let text_bytes t =
    match t.data with
    | Text text -> String.length text.value + String.length text.font_family
    | Path _ | Image _ -> 0
  ;;
end
