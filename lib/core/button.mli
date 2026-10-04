open Core

(** Per-owner button loading and native focus policy. *)
module Focus : sig
  type t =
    | Focusable of Tab_order.t
    | Preserve
  [@@deriving equal, sexp_of]

  (** Normal native focus with the default sequential order. [Preserve] is for
      auxiliary actions: pointer activation preserves sibling focus, and the
      button has no separate Tab/keyboard/semantic Focus target. This differs
      from [Focusable] with [tab_stop=false], which still allows pointer/AX focus. *)
  val default : t
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Loading blocks activation on this owner while preserving normal focus
      eligibility. It does not disable the shared command or own async work.
      Defaults: [loading=false], [focus=Focus.default]. *)
  val create : ?loading:bool -> ?focus:Focus.t -> unit -> t

  val default : t
  val is_loading : t -> bool
  val focus : t -> Focus.t
end

module Expert : sig
  module Presentation : sig
    type t [@@deriving equal, sexp_of]

    val icon_slots : Config.t -> t
    val rich : Config.t -> t
    val config : t -> Config.t
    val to_wire : t -> Gpuio_protocol.Button_wire.Config.t
  end

  val to_wire : Config.t -> Gpuio_protocol.Button_wire.Policy.t
  val of_wire : Gpuio_protocol.Button_wire.Policy.t -> Config.t Or_error.t
end
