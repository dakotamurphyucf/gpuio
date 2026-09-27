(** Declarative native container selection. These values describe rules, not an
    OCaml layout callback. See docs/design/container-queries.md for ownership. *)
open Core

module Branch_id : sig
  type t [@@deriving compare, equal, sexp_of]

  (** 1–128 UTF-8 bytes, without NUL. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Range : sig
  type t [@@deriving equal, sexp_of]

  (** Inclusive minimum (default zero), exclusive optional maximum, in logical
      pixels. Bounds must be finite, nonnegative and strictly ordered. *)
  val create : ?minimum:float -> ?maximum:float -> unit -> t Or_error.t

  val all : t
end

module Predicate : sig
  type t [@@deriving equal, sexp_of]

  (** Width and height are combined with AND; omitted ranges accept every size. *)
  val create : ?width:Range.t -> ?height:Range.t -> unit -> t
end

module Rule : sig
  type t [@@deriving equal, sexp_of]

  val create : condition:Predicate.t -> branch:Branch_id.t -> t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** First matching rule wins; otherwise [default]. At most 32 rules and 16
      distinct branches. Reusing a branch in multiple rules is supported. *)
  val create : default:Branch_id.t -> Rule.t list -> t Or_error.t

  (** Distinct referenced branches in stable first-occurrence order, default
      first. Presentation identity is the branch ID, not this list's index. *)
  val branches : t -> Branch_id.t list

  (** Pure reference selection for tests/reasoning. Rejects nonfinite or negative
      sizes. Native layout evaluates the same rules without calling OCaml. *)
  val select : t -> width:float -> height:float -> Branch_id.t Or_error.t
end

module Selection : sig
  (** Paint-confirmed branch selection. Sizes are logical pixels at selection;
      resizing within the same branch does not emit repeated snapshots. *)
  type t = private
    { branch : Branch_id.t
    ; width : float
    ; height : float
    }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val selection_of_wire
    :  Gpuio_protocol.Container_query_wire.Config.t
    -> Gpuio_protocol.Container_query_wire.Snapshot.t
    -> Selection.t Or_error.t

  val to_wire
    :  Config.t
    -> generation:int64
    -> Gpuio_protocol.Container_query_wire.Config.t Or_error.t
end
