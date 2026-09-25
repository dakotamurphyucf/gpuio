open Core
module Wire = Gpuio_protocol.Canvas_wire

module Point = struct
  type t = Wire.Point.t [@@deriving equal, sexp_of]

  let admit t =
    if Wire.Point.valid t
    then Ok t
    else Or_error.error_string "canvas point must be finite and within +/-1,000,000"
  ;;

  let create ~x ~y = admit { Wire.Point.x; y }
  let x (t : t) = t.x
  let y (t : t) = t.y
end

module Rect = struct
  type t = Wire.Rect.t [@@deriving equal, sexp_of]

  let create ~x ~y ~width ~height =
    let t = { Wire.Rect.x; y; width; height } in
    if Wire.Rect.valid t
    then Ok t
    else
      Or_error.error_string
        "canvas rectangle requires positive bounded dimensions and bounded corners"
  ;;

  let x (t : t) = t.x
  let y (t : t) = t.y
  let width (t : t) = t.width
  let height (t : t) = t.height
  let contains = Wire.Rect.contains
  let intersect = Wire.Rect.intersect
end

module Transform = struct
  type t = Wire.Transform.t [@@deriving equal, sexp_of]

  let identity = Wire.Transform.identity

  let admit t =
    if Wire.Transform.valid t
    then Ok t
    else Or_error.error_string "canvas transform must be finite, bounded and nonsingular"
  ;;

  let create ~a ~b ~c ~d ~tx ~ty = admit { Wire.Transform.a; b; c; d; tx; ty }
  let translate ~x ~y = admit { identity with tx = x; ty = y }
  let scale ~x ~y = admit { identity with a = x; d = y }

  let rotate ~radians =
    if not (Float.is_finite radians)
    then Or_error.error_string "canvas rotation must be finite"
    else (
      let cosine = Float.cos radians
      and sine = Float.sin radians in
      create ~a:cosine ~b:sine ~c:(Float.neg sine) ~d:cosine ~tx:0. ~ty:0.)
  ;;

  let compose t ~local = admit (Wire.Transform.compose t local)
  let apply t point = Point.admit (Wire.Transform.apply t point)

  let unapply t point =
    match Wire.Transform.unapply t point with
    | None -> Or_error.error_string "invalid canvas transform"
    | Some point -> Point.admit point
  ;;
end

module Hit_region = struct
  type t = Wire.Hit_region.t [@@deriving equal, sexp_of]

  let rectangle rect = Wire.Hit_region.Rectangle rect
  let ellipse rect = Wire.Hit_region.Ellipse rect

  let polygon points =
    let t = Wire.Hit_region.Polygon points in
    if Wire.Hit_region.valid t
    then Ok t
    else
      Or_error.error_string
        "canvas polygon requires 3..256 bounded points and non-collinear geometry"
  ;;

  let contains = Wire.Hit_region.contains
  let hit = Wire.Hit_region.hit
end

module Expert = struct
  let point_to_wire t = t
  let rect_to_wire t = t
  let transform_to_wire t = t
  let hit_region_to_wire t = t
end
