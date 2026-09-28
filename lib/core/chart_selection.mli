open Core

(** A semantic target in the immutable publication identified by a chart event.
    IDs are stable; source positions belong to that exact publication. *)
module Span : sig
  type t = private
    { start_index : int
    ; length : int
    ; first : Chart_data.Datum_id.t
    ; last : Chart_data.Datum_id.t
    }
  [@@deriving equal, sexp_of]

  (** The half-open source interval [start_index, start_index + length).
      [first] and [last] identify its endpoints in source order, not an inclusive
      numeric ID range. Length is positive, the endpoint is at most 100,000,
      and endpoint IDs are equal exactly when length is one. *)
  val create
    :  start_index:int
    -> length:int
    -> first:Chart_data.Datum_id.t
    -> last:Chart_data.Datum_id.t
    -> t Or_error.t
end

module Aggregation : sig
  type t =
    | Exact
    | Sum
    | Mean
  [@@deriving equal, sexp_of]
end

type t = private
  | Cartesian of
      { series : Chart_data.Series_id.t
      ; span : Span.t
      ; aggregation : Aggregation.t
      }
  | Slice of Chart_data.Datum_id.t
  | Radar of
      { series : Chart_data.Series_id.t
      ; axis : Chart_data.Datum_id.t
      }
  | Candlestick of
      { span : Span.t
      ; aggregated : bool
      }
  | Node of Chart_data.Node_id.t
  | Edge of Chart_data.Edge_id.t
[@@deriving equal, sexp_of]

(** Exact Cartesian targets contain one datum. Line/area envelope sampling
    selects an original representative, not a fabricated aggregate. Sum/Mean
    identify all source points represented by an aggregated bar, even when its
    source span contains only one point. *)
val cartesian
  :  series:Chart_data.Series_id.t
  -> span:Span.t
  -> aggregation:Aggregation.t
  -> t Or_error.t

val slice : Chart_data.Datum_id.t -> t
val radar : series:Chart_data.Series_id.t -> axis:Chart_data.Datum_id.t -> t

(** [aggregated] identifies native OHLC reduction. An unaggregated candle
    contains one datum. Endpoint IDs never imply numeric ID ordering. *)
val candlestick : span:Span.t -> aggregated:bool -> t Or_error.t

val node : Chart_data.Node_id.t -> t
val edge : Chart_data.Edge_id.t -> t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Chart_selection_wire.t
  val of_wire : Gpuio_protocol.Chart_selection_wire.t -> t Or_error.t
end
