open Core

let within n lo hi = Float.is_finite n && Float.(n >= lo && n <= hi)
let color n = Int64.(n >= 0L && n <= 0xffff_ffffL)

module Brush = struct
  type t =
    | Solid of int64
    | Linear of
        { oklab : bool
        ; angle : float
        ; from : int64
        ; start : float
        ; to_ : int64
        ; stop : float
        }
    | Pattern_slash of int64 * float * float
    | Checkerboard of int64 * float
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Solid n -> color n
    | Pattern_slash (c, width, interval) ->
      color c && within width 0.5 64. && within interval 0.5 64.
    | Checkerboard (c, size) -> color c && within size 0.5 64.
    | Linear { oklab = _; angle; from; start; to_; stop } ->
      within angle 0. 360.
      && color from
      && color to_
      && within start 0. 1.
      && within stop 0. 1.
      && Float.(start <= stop)
  ;;
end

module Corners = struct
  type t =
    { top_left : float
    ; top_right : float
    ; bottom_right : float
    ; bottom_left : float
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    List.for_all [ t.top_left; t.top_right; t.bottom_right; t.bottom_left ] ~f:(fun n ->
      within n 0. 32.)
  ;;
end

module Bar_fill = struct
  type t =
    | Background of Brush.t
    | Base_to_tip of int64 * int64
    | Domain of int64 * int64
    | Values of float * int64 * float * int64
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Background brush -> Brush.valid brush
    | Base_to_tip (a, b) | Domain (a, b) -> color a && color b
    | Values (lo, a, hi, b) ->
      within lo (-1e100) 1e100
      && within hi (-1e100) 1e100
      && Float.(lo < hi)
      && color a
      && color b
  ;;
end

module Stroke = struct
  type t =
    { visible : bool
    ; width : float option
    ; brush : Brush.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.width ~f:(fun n -> within n 0.5 8.) && Brush.valid t.brush
  ;;
end

module Path = struct
  type t =
    { stroke : Stroke.t option
    ; fill : Brush.t option
    ; curve : Chart_options_wire.Curve.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.stroke ~f:Stroke.valid && Option.for_all t.fill ~f:Brush.valid
  ;;
end

module Marker = struct
  type t =
    { visible : bool option
    ; radius : float option
    ; fill : int64 option
    ; stroke : int64 option
    ; stroke_width : float option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.radius ~f:(fun n -> within n 1. 24.)
    && Option.for_all t.fill ~f:color
    && Option.for_all t.stroke ~f:color
    && Option.for_all t.stroke_width ~f:(fun n -> within n 0. 8.)
  ;;
end

module Bar = struct
  type t =
    { fill : Bar_fill.t option
    ; corners : Corners.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Option.for_all t.fill ~f:Bar_fill.valid && Option.for_all t.corners ~f:Corners.valid
  ;;
end

module Series = struct
  type t =
    { series : int64
    ; path : Path.t option
    ; marker : Marker.t option
    ; bar : Bar.t option
    ; legend : int64 option
    ; area_baseline : float option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.series > 0L)
    && Option.for_all t.path ~f:Path.valid
    && Option.for_all t.marker ~f:Marker.valid
    && Option.for_all t.bar ~f:Bar.valid
    && Option.for_all t.legend ~f:color
    && Option.for_all t.area_baseline ~f:(fun n -> within n (-1e100) 1e100)
  ;;
end

module Datum = struct
  type t =
    { series : int64
    ; datum : int64
    ; marker : Marker.t option
    ; bar : Bar.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.series > 0L && t.datum > 0L)
    && Option.for_all t.marker ~f:Marker.valid
    && Option.for_all t.bar ~f:Bar.valid
  ;;
end

module Aggregates = struct
  type t =
    | Inherit_series
    | Uniform
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { series : Series.t list
  ; data : Datum.t list
  ; aggregates : Aggregates.t
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  List.length t.series <= 128
  && List.length t.data <= 1024
  && List.for_all t.series ~f:Series.valid
  && List.for_all t.data ~f:Datum.valid
  && (not
        (List.contains_dup
           (List.map t.series ~f:(fun s -> s.Series.series))
           ~compare:Int64.compare))
  && not
       (List.contains_dup
          (List.map t.data ~f:(fun d -> d.Datum.series, d.datum))
          ~compare:[%compare: int64 * int64])
;;

let empty = { series = []; data = []; aggregates = Aggregates.Inherit_series }
