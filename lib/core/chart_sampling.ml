open Core
module Wire = Gpuio_protocol.Chart_sampling_wire

let buckets max_buckets =
  if max_buckets >= 1 && max_buckets <= 8192
  then Ok (Int64.of_int max_buckets)
  else Or_error.error_string "chart max_buckets must be in [1,8192]"
;;

module Line = struct
  type t = Wire.Line.t [@@deriving equal, sexp_of]

  let exact = Wire.Line.Exact

  let envelope ~max_buckets =
    let%map.Or_error n = buckets max_buckets in
    Wire.Line.Envelope n
  ;;
end

module Bar = struct
  type t = Wire.Bar.t [@@deriving equal, sexp_of]

  let exact = Wire.Bar.Exact

  let sum ~max_buckets =
    let%map.Or_error n = buckets max_buckets in
    Wire.Bar.Sum n
  ;;

  let mean ~max_buckets =
    let%map.Or_error n = buckets max_buckets in
    Wire.Bar.Mean n
  ;;
end

module Candlestick = struct
  type t = Wire.Candlestick.t [@@deriving equal, sexp_of]

  let exact = Wire.Candlestick.Exact

  let ohlc ~max_buckets =
    let%map.Or_error n = buckets max_buckets in
    Wire.Candlestick.Ohlc n
  ;;
end

type t = Wire.t [@@deriving equal, sexp_of]

let create
      ?(line = Wire.Line.Envelope 1024L)
      ?(bars = Bar.exact)
      ?(candles = Candlestick.exact)
      ()
  =
  { Wire.version = 1L; line; bars; candles }
;;

let default = create ()
let line t = t.Wire.line
let bars t = t.Wire.bars
let candles t = t.Wire.candles

module Expert = struct
  let to_wire t = t

  let of_wire t =
    if Wire.valid t then Ok t else Or_error.error_string "invalid chart sampling policy"
  ;;
end
