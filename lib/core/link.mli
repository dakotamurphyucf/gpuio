open Core

(** Configuration foundation for native composed links. Mounting support is still
    under implementation; [Presentation.link] is the existing text-only helper. *)
module Config : sig
  type t [@@deriving equal, sexp_of]

  (** A nonblank UTF-8 accessible [label], at most 4096 bytes and without NUL.
      The label does not supply visible content or a URL. [tab_index] is a signed
      ordering value in -1,000,000..1,000,000; zero is the default. Negative values
      do not implicitly disable traversal: [tab_stop=false] does that explicitly.
      Disabled links cannot activate or receive focus, regardless of tab policy.
      Target routing and opening remain application-owned asynchronous actions. *)
  val create
    :  label:string
    -> ?disabled:bool
    -> ?tab_stop:bool
    -> ?tab_index:int
    -> unit
    -> t Or_error.t

  val label : t -> string
  val is_disabled : t -> bool
  val tab_stop : t -> bool
  val tab_index : t -> int
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Link_wire.t
  val of_wire : Gpuio_protocol.Link_wire.t -> Config.t Or_error.t
end
