open Core

(** Subtree highlighting configuration. Mounted rendering is still in development;
    these constructors alone do not change a view. *)
module Query : sig
  type t [@@deriving equal, sexp_of]

  (** Nonempty literal, <=4096 UTF-8 bytes without NUL. Defaults to Unicode scalar
      lowercasing (not normalization or full case folding). Whole-word requires
      neither neighbour to be alphanumeric or [_]. Matches cannot cross a logical
      group or newline. Whitespace is meaningful. *)
  val create : ?case_sensitive:bool -> ?whole_word:bool -> string -> t Or_error.t

  val text : t -> string
end

module Range : sig
  type t [@@deriving equal, sexp_of]

  (** Half-open, nonempty UTF-8 byte interval. Constructor checks nonnegative,
      ordered endpoints; matching checks source length and scalar boundaries.
      Native documents are excluded from this ordinary-text projection. *)
  val create : start_byte:int64 -> end_byte:int64 -> t Or_error.t

  val start_byte : t -> int64
  val end_byte : t -> int64
end

module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** Resolves colors now. Defaults: theme accent at 30%/65% opacity, radius 2
      logical pixels. Radius must be finite and in 0..64. Recreate on theme change.
      An undefined token is an error; explicit colors preserve their alpha. *)
  val create
    :  ?color:Color.t
    -> ?active_color:Color.t
    -> ?radius:float
    -> ?theme:Theme.t
    -> unit
    -> t Or_error.t

  val default : t
end

module Spec : sig
  type t [@@deriving equal, sexp_of]

  (** Requires a query or ranges. Up to 4096 ranges, in caller order; query matches
      precede ranges. Indices are nonnegative, zero-based match counts, not row or
      byte offsets. [match_index_offset] defaults to 0 for nonvirtualized content.
      Appearance/index changes do not change matcher identity. *)
  val create
    :  ?query:Query.t
    -> ?ranges:Range.t list
    -> ?appearance:Appearance.t
    -> ?active_index:int64
    -> ?match_index_offset:int64
    -> unit
    -> t Or_error.t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Deliberate nested-scope override, not inheritance. *)
  val empty : t

  (** Up to 16 specs and 4096 ranges in total. Spec order is significant.
      The encoded configuration is bounded to 256 KiB. *)
  val create : Spec.t list -> t Or_error.t

  val specs : t -> Spec.t list
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Highlight_wire.Config.t
  val of_wire : Gpuio_protocol.Highlight_wire.Config.t -> Config.t Or_error.t
end
