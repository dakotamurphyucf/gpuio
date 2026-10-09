open Core

(** Presentation controls for ordinary multiline editing. These never replace
    text or reset selection/history. Wrapping changes reflow the native layout;
    enabling wrapping resets horizontal scrolling and offsets may be clamped. *)
module Wrapping_indent : sig
  type t =
    | Flush_left
    | Match_first_line
  [@@deriving equal, sexp_of]
end

type t [@@deriving equal, sexp_of]

val create
  :  ?soft_wrap:bool
  -> ?wrapping_indent:Wrapping_indent.t
  -> ?show_whitespace:bool
  -> ?cursor_margin_lines:int
  -> unit
  -> t Or_error.t

(** Defaults: soft wrap, continuation indent matching the first line, invisible
    whitespace and the native automatic cursor margin. Explicit margins must be
    in 0..256 rendered line heights and are clamped to half the visible viewport. *)
val default : t

val soft_wrap : t -> bool
val wrapping_indent : t -> Wrapping_indent.t
val show_whitespace : t -> bool
val cursor_margin_lines : t -> int option

module Expert : sig
  val to_wire : t -> Gpuio_protocol.Text_area_layout_wire.t
end
