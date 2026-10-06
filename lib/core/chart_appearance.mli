open Core

(** Chart appearance under implementation. Constructors and theme resolution are
    available; attachment to Chart_style and native rendering are not yet wired.
    See docs/design/chart-mark-appearance.md for the complete delivery contract. *)
module Corners : sig
  type t [@@deriving equal, sexp_of]

  (** Physical corners, finite 0..32 logical pixels. Native radii clamp to bounds. *)
  val create
    :  ?top_left:float
    -> ?top_right:float
    -> ?bottom_right:float
    -> ?bottom_left:float
    -> unit
    -> t Or_error.t

  val all : float -> t Or_error.t
end

module Bar_fill : sig
  type t [@@deriving equal, sexp_of]

  (** Verbatim local brush; gradient angles stay physical through orientation changes. *)
  val background : Background.t -> t

  (** sRGB ramp from actual bar base to tip, including negative and stacked bars. *)
  val base_to_tip : from:Color.t -> to_:Color.t -> t

  (** sRGB ramp across the prepared value domain, including stacking/aggregation. *)
  val domain : from:Color.t -> to_:Color.t -> t

  (** sRGB ramp anchored to finite increasing data values within +/-1e100.
      Values outside this range use the respective endpoint color. *)
  val values : from:float * Color.t -> to_:float * Color.t -> t Or_error.t
end

module Stroke : sig
  type t [@@deriving equal, sexp_of]

  (** Width in [0.5,8] logical pixels; omission inherits the chart stroke width. *)
  val create : ?visible:bool -> ?width:float -> Background.t -> t Or_error.t
end

module Path : sig
  type t [@@deriving equal, sexp_of]

  (** Whole-series path styling. Omitted fields inherit existing options/style.
      Explicit fills replace the default area/radar opacity; encode alpha in the brush.
      Stacked area series must share the same effective curve. *)
  val create
    :  ?stroke:Stroke.t
    -> ?fill:Background.t
    -> ?curve:Chart_options.Curve.t
    -> unit
    -> t
end

module Marker : sig
  type t [@@deriving equal, sexp_of]

  (** Partial override; inherited radius remains Chart_style.point_radius. Radius
      1..24, border width 0..8, clamped to radius. Existing dots flag remains master.
      Hidden markers retain the existing line/area/radar selection behavior. *)
  val create
    :  ?visible:bool
    -> ?radius:float
    -> ?fill:Color.t
    -> ?stroke:Color.t
    -> ?stroke_width:float
    -> unit
    -> t Or_error.t
end

module Bar : sig
  type t [@@deriving equal, sexp_of]

  val create : ?fill:Bar_fill.t -> ?corners:Corners.t -> unit -> t
end

module Series : sig
  type t [@@deriving equal, sexp_of]

  (** Series IDs are stable. Applicable fields affect Cartesian/categorical paths,
      bars or markers; radar accepts paths/markers, ignoring curve/bars.
      Legend color is independent; omission preserves the ordinal/palette swatch. *)
  val create
    :  series:Chart_data.Series_id.t
    -> ?path:Path.t
    -> ?marker:Marker.t
    -> ?bar:Bar.t
    -> ?legend:Color.t
    -> unit
    -> t
end

module Datum : sig
  type t [@@deriving equal, sexp_of]

  (** ID scoped to series; radar uses its axis ID. Never an index or category label.
      Unknown IDs and observations omitted by sampling do not create new marks. *)
  val create
    :  series:Chart_data.Series_id.t
    -> datum:Chart_data.Datum_id.t
    -> ?marker:Marker.t
    -> ?bar:Bar.t
    -> unit
    -> t
end

module Aggregates : sig
  type t =
    | Inherit_series
    | Uniform
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

(** <=128 unique series IDs and <=1024 unique (series, datum) pairs. These sparse
    highlights are independent of the source count. They are not a dense style
    channel for every record in a 100k-point source.
    Inherit_series (default) ignores datum overrides on bars representing multiple
    defined observations. Single-observation bars use their datum override.
    Uniform applies a bar override only if every participating observation has the
    same effective bar appearance; otherwise it falls back to the series style.
    Missing categorical observations do not participate. Original values remain
    available through inspection and the data table. *)
val create
  :  ?series:Series.t list
  -> ?data:Datum.t list
  -> ?aggregates:Aggregates.t
  -> unit
  -> t Or_error.t

val empty : t

module Expert : sig
  val to_wire : t -> theme:Theme.t -> Gpuio_protocol.Chart_appearance_wire.t Or_error.t
end
