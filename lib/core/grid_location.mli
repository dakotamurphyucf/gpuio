open Core

(** A complete, atomic native grid placement. Positive lines count from the
    first explicit line; negative lines count from the last. Zero is invalid.
    Placement is resolved against the parent's explicit grid at layout time. *)
module Line : sig
  type t [@@deriving equal, sexp_of]

  val of_int : int -> t Or_error.t
  val of_int_exn : int -> t
  val to_int : t -> int
end

(** A positive track count, at most 1,024. *)
module Span : sig
  type t [@@deriving equal, sexp_of]

  val of_int : int -> t Or_error.t
  val of_int_exn : int -> t
  val to_int : t -> int
end

module Edge : sig
  type t =
    | Auto
    | Line of Line.t
    | Span of Span.t
  [@@deriving equal, sexp_of]
end

module Axis : sig
  type t [@@deriving equal, sexp_of]

  val create : start:Edge.t -> end_:Edge.t -> t
  val auto : t
  val full : t
  val span : Span.t -> t
  val start : t -> Edge.t
  val end_ : t -> Edge.t
end

type t [@@deriving equal, sexp_of]

(** Both axes default to [Axis.auto]. A style replacement replaces both axes.
    Lines have absolute value at most 1,025. Native normalization applies:
    reversed lines swap, equal lines span one track, and two spans use the start
    span. General styles may create implicit tracks; form collections can impose
    stricter bounds against their explicit column count. *)
val create : ?column:Axis.t -> ?row:Axis.t -> unit -> t

val column : t -> Axis.t
val row : t -> Axis.t

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Grid_location_wire.t
end
