open Core

(** Immutable chart data. Constructors validate identities, numeric domains and
    resource bounds before any native registration. No accessor or formatting
    callbacks are retained in these values. Native rendering is a separate layer. *)
module type Id = sig
  type t [@@deriving equal, compare, sexp_of]

  val of_int64 : int64 -> t Or_error.t
  val to_int64 : t -> int64
end

(** Positive identities, stable across updates. Datum IDs are scoped to a series
    for Cartesian data, and to the dataset for slices, candles and radar axes. *)
module Datum_id : Id

module Series_id : Id
module Node_id : Id
module Edge_id : Id
module Category_id : Id

module Point : sig
  type t [@@deriving equal, sexp_of]

  (** Coordinates are finite and have absolute value <= 1e100. [None] is an
      explicit line/area gap, never a NaN sentinel. [label] is a preformatted
      textual description, <=256 UTF-8 bytes, without NUL/CR/LF. *)
  val create
    :  id:Datum_id.t
    -> x:float
    -> y:float option
    -> ?label:string
    -> unit
    -> t Or_error.t

  val id : t -> Datum_id.t
  val x : t -> float
  val y : t -> float option
  val label : t -> string
end

module Series : sig
  type t [@@deriving equal, sexp_of]

  (** Nonblank name <=128 UTF-8 bytes, without NUL/CR/LF. Points have unique
      datum IDs and strictly increasing x coordinates; empty series are valid.
      One dataset admits <=32 series and <=100,000 points in total. *)
  val create : id:Series_id.t -> name:string -> Point.t list -> t Or_error.t

  val id : t -> Series_id.t
  val name : t -> string
  val points : t -> Point.t list
end

module Layer : sig
  type t =
    | Line of Series.t
    | Area of Series.t
    | Bar of Series.t
  [@@deriving equal, sexp_of]
end

(** Categorical identity is independent of label text and position. Labels are
    nonblank, <=256 UTF-8 bytes without NUL/CR/LF. Equal labels are permitted. *)
module Category : sig
  type t [@@deriving equal, sexp_of]

  val create : id:Category_id.t -> label:string -> t Or_error.t
  val id : t -> Category_id.t
  val label : t -> string
end

module Categorical_point : sig
  type t [@@deriving equal, sexp_of]

  (** [None] is a missing observation, including for categorical bars. Present
      values have the same finite numeric bounds as [Point]. *)
  val create
    :  id:Datum_id.t
    -> category:Category_id.t
    -> value:float option
    -> ?label:string
    -> unit
    -> t Or_error.t

  val id : t -> Datum_id.t
  val category : t -> Category_id.t
  val value : t -> float option
  val label : t -> string
end

module Categorical_series : sig
  type t [@@deriving equal, sexp_of]

  (** Bounded nonblank name, unique datum IDs and unique category IDs. Domain
      membership/order is validated when constructing the whole dataset. *)
  val create : id:Series_id.t -> name:string -> Categorical_point.t list -> t Or_error.t

  val id : t -> Series_id.t
  val name : t -> string
  val points : t -> Categorical_point.t list
end

module Categorical_layer : sig
  type t =
    | Line of Categorical_series.t
    | Area of Categorical_series.t
    | Bar of Categorical_series.t
  [@@deriving equal, sexp_of]
end

module Slice : sig
  type t [@@deriving equal, sexp_of]

  (** Values are finite, nonnegative and <=1e100. Zero-valued slices remain in
      the textual data alternative but have no painted area. *)
  val create : id:Datum_id.t -> label:string -> value:float -> t Or_error.t

  val id : t -> Datum_id.t
  val label : t -> string
  val value : t -> float
end

module Radar_axis : sig
  type t [@@deriving equal, sexp_of]

  (** Positive finite maximum <=1e100; per-axis domains avoid silently comparing
      values measured in different units on an arbitrary shared scale. *)
  val create : id:Datum_id.t -> label:string -> maximum:float -> t Or_error.t

  val id : t -> Datum_id.t
  val label : t -> string
  val maximum : t -> float
end

module Radar_series : sig
  type t [@@deriving equal, sexp_of]

  val create : id:Series_id.t -> name:string -> (Datum_id.t * float) list -> t Or_error.t
  val id : t -> Series_id.t
  val name : t -> string
  val values : t -> (Datum_id.t * float) list
end

module Candle : sig
  type t [@@deriving equal, sexp_of]

  (** All coordinates are finite and bounded as for [Point]. Negative prices
      are valid; [low <= min(open,close) <= max(open,close) <= high] is required.
      A zero-height candle is valid. *)
  val create
    :  id:Datum_id.t
    -> x:float
    -> label:string
    -> open_:float
    -> high:float
    -> low:float
    -> close:float
    -> t Or_error.t

  val id : t -> Datum_id.t
  val x : t -> float
  val label : t -> string
  val open_ : t -> float
  val high : t -> float
  val low : t -> float
  val close : t -> float
end

module Node : sig
  type t [@@deriving equal, sexp_of]

  val create : id:Node_id.t -> label:string -> t Or_error.t
  val id : t -> Node_id.t
  val label : t -> string
end

module Edge : sig
  type t [@@deriving equal, sexp_of]

  val create
    :  id:Edge_id.t
    -> source:Node_id.t
    -> target:Node_id.t
    -> value:float
    -> t Or_error.t

  val id : t -> Edge_id.t
  val source : t -> Node_id.t
  val target : t -> Node_id.t
  val value : t -> float
end

type t [@@deriving equal, sexp_of]

(** Line/area accept missing values as gaps; bars require every y value. Numeric
    x coordinates determine positions; labels never act as identity or scale.
    Empty input is valid. These constructors perform no implicit aggregation. *)
val line : Series.t list -> t Or_error.t

val area : Series.t list -> t Or_error.t
val bar : Series.t list -> t Or_error.t

(** Shared numeric coordinates allow mixed line/area/bar layers. All series IDs
    remain unique across layers. Zero is the area/bar baseline. *)
val cartesian : Layer.t list -> t Or_error.t

(** The explicit category list defines order, not numeric ID or label ordering.
    Every series supplies one point per category in that order; [None] explicitly
    records a missing value. No sorting, zero-filling or insertion occurs.
    <=100,000 categories, <=32 series, <=100,000 points across all series and
    <=8 MiB total text, including category labels. *)
val categorical : categories:Category.t list -> Categorical_layer.t list -> t Or_error.t

(** At most 256 slices with unique IDs and nonblank labels <=256 UTF-8 bytes. *)
val pie : Slice.t list -> t Or_error.t

(** <=64 unique axes and <=32 unique series. A nonempty radar requires >=3 axes.
    Each series must provide exactly one value for each axis, in any order, in
    [0, axis.maximum]. Empty axes with empty series is a valid empty chart. *)
val radar : axes:Radar_axis.t list -> Radar_series.t list -> t Or_error.t

(** <=100,000 unique candles with strictly increasing x coordinates. *)
val candlestick : Candle.t list -> t Or_error.t

(** <=256 unique nodes, <=2048 unique edges. Every edge references existing
    distinct nodes, values are nonnegative, and the graph must be acyclic.
    Isolated nodes, parallel edges with distinct IDs, zero flows and empty graphs
    are valid. No flow conservation or hidden aggregation is imposed. *)
val sankey : nodes:Node.t list -> edges:Edge.t list -> t Or_error.t

(** Number of source data values, including gaps/zero values. No downsampling has
    happened at this boundary. Total retained label/name text is <=8 MiB. *)
val value_count : t -> int

val text_bytes : t -> int

module Expert : sig
  (** Read-only inspection for the native protocol and data-table adapters.
      These variants do not provide an unchecked constructor for [t]. *)
  type contents =
    | Cartesian of Layer.t list
    | Pie of Slice.t list
    | Radar of Radar_axis.t list * Radar_series.t list
    | Candlestick of Candle.t list
    | Sankey of Node.t list * Edge.t list
    | Categorical of Category.t list * Categorical_layer.t list
  [@@deriving equal, sexp_of]

  val contents : t -> contents

  (** Conservative registration charge for the immutable data, conversion records
      and bounded encode buffers. Shared identical values can be charged once;
      this is an admission estimate, not an OCaml heap/RSS measurement. *)
  val retained_bytes : t -> int

  (** Raw wire conversion does not bypass the validating domain constructors. *)
  val to_wire : t -> Gpuio_protocol.Chart_data_wire.t

  val of_wire : Gpuio_protocol.Chart_data_wire.t -> t Or_error.t

  (** Versioned standalone bin_prot data, <=16 MiB. Decoding bounds aggregate
      lists/text before allocation, rejects trailing bytes and validates semantics. *)
  val encode : t -> string Or_error.t

  val decode : string -> t Or_error.t
end
