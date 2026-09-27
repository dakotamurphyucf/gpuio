open Core

(** Source-independent native plotting options. Geometry uses logical pixels;
    colors, theme and the accessible description belong to the mounted view.
    Options for other dataset families are retained but have no effect. No
    formatting or plotting operation calls back synchronously into OCaml. *)
module Number_format : sig
  type t [@@deriving equal, sexp_of]

  (** Compact uses decimal K/M/B/T suffixes and scientific notation beyond that
      range; precision constructors accept 0..6 decimal places. Percent displays
      [100 * value] followed by [%], without changing the data or scale. *)
  val compact : t

  val fixed : decimals:int -> t Or_error.t
  val scientific : decimals:int -> t Or_error.t
  val percent : decimals:int -> t Or_error.t
end

module Axes : sig
  type t [@@deriving equal, sexp_of]

  (** Numeric linear domains come from the complete displayed dataset, including
      explicitly aggregated values. Bar/area include zero. Empty domains use
      [0,1]; constant domains place the value in the center. [ticks] is 2..12.
      Source labels are used for tooltips/data access, not categorical spacing. *)
  val create
    :  ?x:bool
    -> ?y:bool
    -> ?grid:bool
    -> ?ticks:int
    -> ?x_format:Number_format.t
    -> ?y_format:Number_format.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Curve : sig
  type t =
    | Linear
    | Natural
    | Step_after
  [@@deriving equal, sexp_of]
end

module Orientation : sig
  type t =
    | Vertical
    | Horizontal
  [@@deriving equal, sexp_of]
end

module Cartesian : sig
  type t [@@deriving equal, sexp_of]

  (** Orientation applies to every layer of a mixed plot. Bars are grouped by
      layer, never implicitly stacked. [bar_width] in (0,1] is the fraction of
      nearest distinct x spacing occupied by each group. Curve defaults to
      Linear; Natural is an interpolating cubic spline and can overshoot values.
      Missing values always split paths. Singleton runs remain visible even
      when [dots=false]. *)
  val create
    :  ?curve:Curve.t
    -> ?dots:bool
    -> ?orientation:Orientation.t
    -> ?bar_width:float
    -> unit
    -> t Or_error.t

  val default : t
end

module Pie : sig
  type t [@@deriving equal, sexp_of]

  (** [inner_radius] is the donut hole fraction in [0,0.95]; [pad_angle] is
      radians in [0,0.2]. Padding is clamped per slice to avoid negative wedges.
      Zero-valued slices have no area; an all-zero pie has no wedges. *)
  val create
    :  ?inner_radius:float
    -> ?pad_angle:float
    -> ?labels:bool
    -> unit
    -> t Or_error.t

  val default : t
end

module Radar : sig
  type t [@@deriving equal, sexp_of]

  (** Axis maxima come from the dataset. [levels] is 1..12. *)
  val create : ?levels:int -> ?dots:bool -> ?labels:bool -> unit -> t Or_error.t

  val default : t
end

module Candlestick : sig
  type t [@@deriving equal, sexp_of]

  (** [body_width] in (0,1] is the fraction of nearest x spacing. Rising candles
      are hollow, falling candles filled, equal open/close a horizontal mark;
      movement is therefore not encoded by color alone. *)
  val create : ?body_width:float -> unit -> t Or_error.t

  val default : t
end

module Sankey : sig
  module Alignment : sig
    type t =
      | Left
      | Right
      | Center
      | Justify
    [@@deriving equal, sexp_of]
  end

  module Scale : sig
    type t =
      | Linear
      | Sqrt
    [@@deriving equal, sexp_of]
  end

  type t [@@deriving equal, sexp_of]

  (** Node width 1..64 logical pixels, padding 0..64, relaxation iterations
      0..32. Small plots clamp width/padding to fit their bounds. Sqrt changes
      geometric weights only; tooltips and events retain original flow values.
      Zero-flow and isolated nodes remain available in the data alternative. *)
  val create
    :  ?node_width:float
    -> ?node_padding:float
    -> ?alignment:Alignment.t
    -> ?scale:Scale.t
    -> ?iterations:int
    -> ?labels:bool
    -> unit
    -> t Or_error.t

  val default : t
end

type t [@@deriving equal, sexp_of]

val create
  :  ?axes:Axes.t
  -> ?cartesian:Cartesian.t
  -> ?pie:Pie.t
  -> ?radar:Radar.t
  -> ?candlestick:Candlestick.t
  -> ?sankey:Sankey.t
  -> unit
  -> t

val default : t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Chart_options_wire.t
  val of_wire : Gpuio_protocol.Chart_options_wire.t -> t Or_error.t
end
