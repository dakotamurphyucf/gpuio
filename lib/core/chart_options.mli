open Core

(** Source-independent native plotting options. Geometry uses logical pixels;
    colors, theme and the accessible description belong to the mounted view.
    Options for other dataset families are retained but have no effect. No
    formatting or plotting operation calls back synchronously into OCaml.
    Native layout reserves space for the data control, axes and legend. Series
    identifiers repeat at bounded representative positions in multi-series
    Cartesian/radar plots; dense or coincident values can still overlap. *)
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
      Numeric point labels do not determine spacing. Categorical datasets use
      their explicit domain labels/positions instead of [x_format]. *)
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
  (** Reversed variants reverse the numeric value axis, preserving category
      order. Positive bars grow downward or leftward respectively. Axis gutters
      remain in their usual positions. Applies to Cartesian layers only. *)
  type t =
    | Vertical
    | Horizontal
    | Vertical_reversed
    | Horizontal_reversed
  [@@deriving equal, sexp_of]
end

(** Native categorical projection. Ignored for numeric datasets. Auto uses
    point spacing without bars and band spacing with any bar layer. *)
module Category_layout : sig
  type t [@@deriving equal, sexp_of]

  val auto : t

  (** Padding in [0,1] step units, default 0. Singleton categories are centered.
      With bars, effective padding is at least 0.5 to fit endpoint groups. *)
  val point : ?padding:float -> unit -> t Or_error.t

  (** Inner padding in [0,1), default 0.2; outer in [0,1], default 0.1.
      Mixed line/area layers use band centers; bars group within each band. *)
  val band : ?inner_padding:float -> ?outer_padding:float -> unit -> t Or_error.t
end

module Stacking : sig
  (** Grouped is the default. Stacked accumulates bars with bars and areas with
      areas, independently in source layer order; line overlays are unchanged.
      Values accumulate algebraically, including negatives (no normalization or
      separate positive/negative stacks). Missing values contribute zero to later
      baselines but retain their own gaps and original missing observations.
      Numeric layers within each stack family require identical x positions and
      lengths; native preparation rejects misalignment. Categorical layers are
      already aligned to their explicit domain. *)
  type t =
    | Grouped
    | Stacked
  [@@deriving equal, sexp_of]
end

module Cartesian : sig
  type t [@@deriving equal, sexp_of]

  (** Orientation applies to every layer of a mixed plot. Bars are grouped by
      layer unless [stacking=Stacked]. [bar_width] in (0,1] is the fraction of
      nearest distinct numeric x spacing occupied by each group. For categorical
      data it is the fraction of the available category band. Curve defaults to
      Linear; Natural is an interpolating cubic spline and can overshoot values.
      Missing values always split paths. Singleton runs remain visible even
      when [dots=false]. *)
  val create
    :  ?curve:Curve.t
    -> ?dots:bool
    -> ?orientation:Orientation.t
    -> ?bar_width:float
    -> ?category_layout:Category_layout.t
    -> ?stacking:Stacking.t
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
