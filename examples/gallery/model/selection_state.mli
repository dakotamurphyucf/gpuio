(** Controlled selection examples. Requests are applied to the current model;
    disabling the preview rejects requests that were already queued. *)
module Format : sig
  type t =
    | Bold
    | Italic
    | Monospace
  [@@deriving equal, sexp_of]

  val all : t list

  (** Stable command identity, independent of the displayed label. *)
  val command_id : t -> Gpuio.Command.Id.t

  val label : t -> string
end

module Alignment : sig
  type t =
    | Left
    | Center
    | Right
  [@@deriving equal, sexp_of]

  val all : t list

  (** Stable command identity, independent of the displayed label. *)
  val command_id : t -> Gpuio.Command.Id.t

  val label : t -> string
end

type t

module Action : sig
  type t =
    | Toggle_enabled
    | Toggle_italic_enabled
    | Toggle_bold_loading
    | Toggle_format of Format.t
    | Align of Alignment.t
    | Toggle_all
end

val initial : t

(** Bulk requests change only available items and preserve all other selections. *)
val apply : t -> Action.t -> t

val enabled : t -> bool
val italic_enabled : t -> bool
val bold_loading : t -> bool

(** Command availability excludes per-owner loading. Loading preserves native
    focus, while the current-model reducer still rejects requests for that item. *)
val format_enabled : t -> Format.t -> bool

val format_loading : t -> Format.t -> bool
val selected : t -> Format.t -> bool
val alignment : t -> Alignment.t

(** Reports the complete selection, including unavailable items. *)
val master : t -> Gpuio.Check_state.t
