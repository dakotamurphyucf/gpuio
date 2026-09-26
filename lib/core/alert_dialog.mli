open Core

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** An urgent, application-controlled modal with alert-dialog accessibility
      semantics. Backdrop clicks never dismiss it. Escape defaults to enabled;
      its dismissal request does not confirm an action or close the dialog.
      [width] defaults to 480 logical pixels, with the same bounds and styling
      rules as [Overlay.Config]. Supply a concise accessible label. *)
  val create
    :  label:string
    -> ?width:float
    -> ?dismiss_on_escape:bool
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val overlay : Config.t -> Overlay.Config.t
end
