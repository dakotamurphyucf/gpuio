open Core

module Line = struct
  type t =
    | Exact
    | Envelope of int64
  [@@deriving bin_io, equal, sexp_of]
end

module Bar = struct
  type t =
    | Exact
    | Sum of int64
    | Mean of int64
  [@@deriving bin_io, equal, sexp_of]
end

module Candlestick = struct
  type t =
    | Exact
    | Ohlc of int64
  [@@deriving bin_io, equal, sexp_of]
end

type t =
  { version : int64
  ; line : Line.t
  ; bars : Bar.t
  ; candles : Candlestick.t
  }
[@@deriving bin_io, equal, sexp_of]

let valid_buckets n = Int64.(n >= 1L && n <= 8192L)

let valid t =
  Int64.equal t.version 1L
  && (match t.line with
      | Exact -> true
      | Envelope n -> valid_buckets n)
  && (match t.bars with
      | Exact -> true
      | Sum n | Mean n -> valid_buckets n)
  &&
  match t.candles with
  | Exact -> true
  | Ohlc n -> valid_buckets n
;;
