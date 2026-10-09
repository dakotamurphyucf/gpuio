open Core

module Span = struct
  type t =
    { start_index : int64
    ; length : int64
    ; first : int64
    ; last : int64
    }
  [@@deriving bin_io, compare, equal, sexp_of]

  let valid t =
    Int64.(
      t.start_index >= 0L
      && t.start_index < 100_000L
      && t.length > 0L
      && t.length <= 100_000L - t.start_index
      && t.first > 0L
      && t.last > 0L
      && Bool.equal (t.length = 1L) (t.first = t.last))
  ;;
end

module Aggregation = struct
  type t =
    | Exact
    | Sum
    | Mean
  [@@deriving bin_io, compare, equal, sexp_of]
end

type t =
  | Cartesian of
      { series : int64
      ; span : Span.t
      ; aggregation : Aggregation.t
      }
  | Slice of int64
  | Radar of
      { series : int64
      ; axis : int64
      }
  | Candlestick of
      { span : Span.t
      ; aggregated : bool
      }
  | Node of int64
  | Edge of int64
[@@deriving bin_io, compare, equal, sexp_of]

let valid = function
  | Cartesian { series; span; aggregation } ->
    Int64.(series > 0L)
    && Span.valid span
    &&
      (match aggregation with
      | Exact -> Int64.equal span.length 1L
      | Sum | Mean -> true)
  | Slice id | Node id | Edge id -> Int64.(id > 0L)
  | Radar { series; axis } -> Int64.(series > 0L && axis > 0L)
  | Candlestick { span; aggregated } ->
    Span.valid span && (aggregated || Int64.equal span.length 1L)
;;
