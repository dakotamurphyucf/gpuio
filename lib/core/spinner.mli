open Core

(** Configuration for a native indeterminate spinner. Configuration construction
    performs no I/O and does not decode or acquire the icon registration. *)
module Config : sig
  type t [@@deriving equal, sexp_of]

  (** [label] is nonblank UTF-8 without NUL, at most 4096 bytes. [period] defaults
      to 800ms, accepts 100ms..60s and rounds up to a whole millisecond.
      [animated] defaults to true; static and reduced-motion presentations retain
      recognizable artwork. [easing] defaults to [Animation.Easing.ease_in_out].
      Easing is declarative; it never calls OCaml on a native animation frame.

      [icon], when supplied, must declare SVG format. Its alpha supplies a
      decorative monochrome icon tinted by the inherited foreground. Loading or
      failed icons use built-in artwork. The spinner label is its sole semantic
      label. A handle does not retain a registration; mounted native bindings
      acquire their own leases. Cross-application handles retain normal image
      error semantics rather than becoming references in another application. *)
  val create
    :  label:string
    -> ?icon:Asset.Handle.t
    -> ?easing:Animation.Easing.t
    -> ?period:Time_ns.Span.t
    -> ?animated:bool
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val image : Config.t -> Image.Config.t option
  val loading : Config.t -> Loading.Config.t

  val to_wire
    :  Config.t
    -> owner:Asset.Expert.Owner.t option
    -> Gpuio_protocol.Spinner_wire.Config.t
end
