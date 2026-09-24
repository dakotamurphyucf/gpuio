open Core

(** Dismissal is a request. The application closes an overlay by removing its
    content; native focus policy remains active until that update is accepted. *)
module Dismissal : sig
  type t =
    | Escape
    | Outside_pointer
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** [width] is the desired panel width in logical pixels, clamped to the window.
      Defaults: width 480, Escape enabled, outside-pointer dismissal disabled.
      Supply an accessible label and ordinary styled views as the content.
      Explicit panel style dimensions override the default geometry. [placement]
      applies to popovers; centered dialogs ignore it. *)
  val create
    :  label:string
    -> ?width:float
    -> ?placement:Placement.t
    -> ?dismiss_on_escape:bool
    -> ?dismiss_on_outside_pointer:bool
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val placement : Config.t -> Placement.t

  val to_wire
    :  Config.t
    -> kind:Gpuio_protocol.Wire.Overlay_kind.t
    -> Gpuio_protocol.Wire.Overlay.t

  val allows : Config.t -> Dismissal.t -> bool
end
