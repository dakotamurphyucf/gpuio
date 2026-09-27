open Core

(** Explicit source-to-geometry policies. Source datasets and IDs are never
    modified. Buckets partition the shared numeric x domain, rather than input
    array positions. Effective buckets are bounded by the plot's logical-pixel
    width and [max_buckets], which must be in [1,8192]. *)
module Line : sig
  type t [@@deriving equal, sexp_of]

  val exact : t

  (** Retains the first, minimum-y, maximum-y and last source points of each
      bucket, in source order without duplicates. Each contiguous defined run is
      reduced independently: missing values always break paths, including gaps
      within a single bucket. Retained points keep their original datum IDs.
      Many short runs can therefore retain more than four points per bucket. *)
  val envelope : max_buckets:int -> t Or_error.t
end

module Bar : sig
  type t [@@deriving equal, sexp_of]

  val exact : t

  (** Explicit aggregation, independently per series. Each output keeps its
      contiguous source index range (and thus original IDs at that revision).
      X is the midpoint of the first/last source x; sum may exceed the source
      value bound, but remains finite within the bounded source count. *)
  val sum : max_buckets:int -> t Or_error.t

  val mean : max_buckets:int -> t Or_error.t
end

module Candlestick : sig
  type t [@@deriving equal, sexp_of]

  val exact : t

  (** Open is the first source open, close the last close, high the maximum
      source high and low the minimum source low. Source ranges are preserved;
      candles never use line-envelope sampling. *)
  val ohlc : max_buckets:int -> t Or_error.t
end

type t [@@deriving equal, sexp_of]

(** Defaults: line/area envelope with at most 1024 buckets; exact bars and
    candles. Pie, radar and Sankey always retain their complete bounded data.
    These policies do not promise an unbounded rendering budget: later geometry
    admission may still report a limit rather than silently dropping values. *)
val create : ?line:Line.t -> ?bars:Bar.t -> ?candles:Candlestick.t -> unit -> t

val default : t
val line : t -> Line.t
val bars : t -> Bar.t
val candles : t -> Candlestick.t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Chart_sampling_wire.t
  val of_wire : Gpuio_protocol.Chart_sampling_wire.t -> t Or_error.t
end
