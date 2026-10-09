open Core

(** Paint coordination for a primary action and a native menu trigger. Each
    action retains its own focus, accessibility and activation owner. *)
module Appearance : sig
  type t [@@deriving equal, sexp_of]

  (** [surface] paints each available half while the pair is hovered or its menu
      is open. [menu_open] additionally paints the menu trigger while open.
      Both default to empty. Ordinary per-half hover/pressed styles take
      precedence; disabled/loading halves do not receive coordination styles.

      Only Base declarations of Background, Foreground, Border_color, Shadows
      and Text_decoration are accepted, with at most 64 declarations total.
      Layout, clipping, opacity and input policy remain on the ordinary views. *)
  val create : ?surface:Style.t -> ?menu_open:Style.t -> unit -> t Or_error.t

  val default : t
end

module Expert : sig
  val to_wire
    :  Appearance.t
    -> parts:Gpuio_protocol.Wire.Split_button.Parts.t
    -> theme:Theme.t
    -> Gpuio_protocol.Wire.Split_button.t Or_error.t
end
