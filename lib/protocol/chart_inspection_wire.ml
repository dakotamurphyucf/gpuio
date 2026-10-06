open Core

module Placement = struct
  type t =
    | Corner
    | Anchor
    | Cursor
  [@@deriving bin_io, equal, sexp_of]
end

module Axis = struct
  type t =
    | Off
    | Vertical
    | Horizontal
    | Both
  [@@deriving bin_io, equal, sexp_of]
end

module Pattern = struct
  type t =
    | Dashed
    | Solid
  [@@deriving bin_io, equal, sexp_of]
end

let within n a b = Float.is_finite n && Float.(n >= a && n <= b)
let color n = Int64.(n >= 0L && n <= 0xffff_ffffL)

module Card = struct
  type t =
    { visible : bool
    ; title : bool
    ; values : bool
    ; placement : Placement.t
    ; width : float
    ; gap : float
    ; padding : float
    ; radius : float
    ; font_size : float
    ; line_height : float
    ; border_width : float
    ; text_color : int64 option
    ; background : int64 option
    ; border_color : int64 option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    within t.width 96.0 480.0
    && within t.gap 0.0 64.0
    && within t.padding 0.0 24.0
    && within t.radius 0.0 24.0
    && within t.font_size 8.0 32.0
    && within t.line_height 8.0 48.0
    && within t.border_width 0.0 8.0
    && Option.for_all t.text_color ~f:color
    && Option.for_all t.background ~f:color
    && Option.for_all t.border_color ~f:color
    && Float.(t.line_height >= t.font_size)
  ;;
end

module Crosshair = struct
  type t =
    { axis : Axis.t
    ; pattern : Pattern.t
    ; thickness : float
    ; color : int64 option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = within t.thickness 0.5 64.0 && Option.for_all t.color ~f:color
end

module Marker = struct
  type t =
    { visible : bool
    ; status : bool
    ; size : float
    ; stroke_width : float
    ; fill : int64 option
    ; stroke : int64 option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    within t.size 2.0 48.0
    && within t.stroke_width 0.0 8.0
    && Option.for_all t.fill ~f:color
    && Option.for_all t.stroke ~f:color
    && Float.(t.stroke_width <= t.size /. 2.)
  ;;
end

type t =
  { card : Card.t
  ; crosshair : Crosshair.t
  ; marker : Marker.t
  }
[@@deriving bin_io, equal, sexp_of]

let valid t = Card.valid t.card && Crosshair.valid t.crosshair && Marker.valid t.marker
