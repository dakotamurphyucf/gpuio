open Core

type t =
  | Light
  | Dark
[@@deriving equal, sexp_of]

val toggle : t -> t
val label : t -> string

module Preference : sig
  type appearance = t [@@deriving equal, sexp_of]

  type t =
    | System
    | Explicit of appearance
  [@@deriving equal, sexp_of]

  (** Resolve the application palette independently of native decorations. System
      uses Dark until the initial native observation arrives. Explicit choices
      ignore later native appearance changes. *)
  val resolve : t -> native:Gpuio.Window.Appearance.t option -> appearance
end

module Scale : sig
  type t =
    | Compact
    | Comfortable
    | Large
  [@@deriving equal, sexp_of]

  val all : t list
  val next : t -> t
  val label : t -> string
  val factor : t -> float
end
