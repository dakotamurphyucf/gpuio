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

module Appearance : sig
  (** Appearance changes preserve native open/highlight/focus state. Geometry is
      explicit so virtualization remains uniform. Styles accept colors, opacity,
      corners, shadows, text presentation and cursor; structural/interaction
      properties are rejected. Popup supports Base/Hovered; options additionally
      support Focused (native highlight), Pressed, Selected and Disabled; empty
      state supports Base only. Theme tokens resolve with the application theme. *)
  type t [@@deriving equal, sexp_of]

  val default : t

  (** Positive finite logical pixel dimensions, at most 1,000,000. Visible rows
      require 1..64. Empty text requires 1..1024 UTF-8 bytes without NUL. At most
      128 style declarations across all parts. Native geometry clamps to window. *)
  val create
    :  ?popup_width:float
    -> ?row_height:float
    -> ?max_visible_rows:int
    -> ?empty_label:string
    -> ?popup_style:Style.t
    -> ?option_style:Style.t
    -> ?empty_style:Style.t
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val appearance_to_wire
    :  Appearance.t
    -> theme:Theme.t
    -> Gpuio_protocol.Wire.Choice_appearance.t Or_error.t

  val config_to_wire : Config.t -> Gpuio_protocol.Wire.Choice.Config.t
end
