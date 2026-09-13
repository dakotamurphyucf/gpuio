open Core

module Id : sig
  (** Stable, case-sensitive choice identity. Labels and collection positions
      may change without changing this identity. IDs cross the native bridge. *)
  type t [@@deriving equal, compare, sexp_of]

  (** Requires 1..256 UTF-8 bytes without NUL. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

(** An immutable option shared by radio groups, selects and comboboxes. *)
type t [@@deriving equal, sexp_of]

(** Labels require 1..4096 UTF-8 bytes without NUL. Disabled options may remain
    selected by application state, but cannot be chosen by native activation. *)
val create : id:Id.t -> label:string -> ?disabled:bool -> unit -> t Or_error.t

val id : t -> Id.t
val label : t -> string
val is_disabled : t -> bool

module Collection : sig
  type item = t
  type t [@@deriving equal, sexp_of]

  val max_choices : int
  val max_text_bytes : int

  (** Preserve order; reject duplicate IDs and oversized collections. Equal
      labels are permitted. Empty collections are valid. *)
  val create : item list -> t Or_error.t

  val to_list : t -> item list
  val find : t -> Id.t -> item option

  (** Absence is valid. A present selection must name a member, including a
      disabled member. Filtering the displayed options need not clear selection. *)
  val validate_selection : t -> Id.t option -> unit Or_error.t
end

module Config : sig
  (** Validated application-owned configuration. An absent selection and a
      disabled selected option are both permitted; native activation only
      requests enabled options. *)
  type t [@@deriving equal, sexp_of]

  val create
    :  label:string
    -> options:Collection.t
    -> selected:Id.t option
    -> ?disabled:bool
    -> unit
    -> t Or_error.t

  val label : t -> string
  val options : t -> Collection.t
  val selected : t -> Id.t option
  val is_disabled : t -> bool
  val can_select : t -> Id.t -> bool
end

module Expert : sig
  val config_to_wire : Config.t -> Gpuio_protocol.Wire.Choice.Config.t
end
