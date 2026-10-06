open Core

module Number_format = struct
  type t =
    | Compact
    | Fixed of int64
    | Scientific of int64
    | Percent of int64
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Compact -> true
    | Fixed n | Scientific n | Percent n -> Int64.(n >= 0L && n <= 6L)
  ;;
end

module Axes = struct
  type t =
    { x : bool
    ; y : bool
    ; grid : bool
    ; ticks : int64
    ; x_format : Number_format.t
    ; y_format : Number_format.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.ticks >= 2L && t.ticks <= 12L)
    && Number_format.valid t.x_format
    && Number_format.valid t.y_format
  ;;
end

module Curve = struct
  type t =
    | Linear
    | Natural
    | Step_after
  [@@deriving bin_io, equal, sexp_of]
end

module Orientation = struct
  type t =
    | Vertical
    | Horizontal
    | Vertical_reversed
    | Horizontal_reversed
  [@@deriving bin_io, equal, sexp_of]
end

let between n lo hi = Float.is_finite n && Float.(n >= lo && n <= hi)
let fraction n = between n 0. 1. && Float.(n > 0.)

module Category_layout = struct
  type t =
    | Auto
    | Point of float
    | Band of
        { inner : float
        ; outer : float
        }
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Auto -> true
    | Point p -> between p 0. 1.
    | Band { inner; outer } ->
      between inner 0. 1. && Float.(inner < 1.) && between outer 0. 1.
  ;;
end

module Stacking = struct
  type t =
    | Grouped
    | Stacked
  [@@deriving bin_io, equal, sexp_of]
end

module Cartesian = struct
  type t =
    { curve : Curve.t
    ; dots : bool
    ; orientation : Orientation.t
    ; bar_width : float
    ; category_layout : Category_layout.t
    ; stacking : Stacking.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = fraction t.bar_width && Category_layout.valid t.category_layout
end

module Pie = struct
  type t =
    { inner_radius : float
    ; pad_angle : float
    ; labels : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = between t.inner_radius 0. 0.95 && between t.pad_angle 0. 0.2
end

module Radar = struct
  type t =
    { levels : int64
    ; dots : bool
    ; labels : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = Int64.(t.levels >= 1L && t.levels <= 12L)
end

module Candlestick = struct
  type t = { body_width : float } [@@deriving bin_io, equal, sexp_of]

  let valid t = fraction t.body_width
end

module Sankey = struct
  module Alignment = struct
    type t =
      | Left
      | Right
      | Center
      | Justify
    [@@deriving bin_io, equal, sexp_of]
  end

  module Scale = struct
    type t =
      | Linear
      | Sqrt
    [@@deriving bin_io, equal, sexp_of]
  end

  type t =
    { node_width : float
    ; node_padding : float
    ; alignment : Alignment.t
    ; scale : Scale.t
    ; iterations : int64
    ; labels : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    between t.node_width 1. 64.
    && between t.node_padding 0. 64.
    && Int64.(t.iterations >= 0L && t.iterations <= 32L)
  ;;
end

type t =
  { version : int64
  ; axes : Axes.t
  ; cartesian : Cartesian.t
  ; pie : Pie.t
  ; radar : Radar.t
  ; candlestick : Candlestick.t
  ; sankey : Sankey.t
  }
[@@deriving bin_io, equal, sexp_of]

let valid t =
  Int64.equal t.version 3L
  && Axes.valid t.axes
  && Cartesian.valid t.cartesian
  && Pie.valid t.pie
  && Radar.valid t.radar
  && Candlestick.valid t.candlestick
  && Sankey.valid t.sankey
;;
