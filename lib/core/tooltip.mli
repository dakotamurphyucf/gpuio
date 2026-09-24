open Core

module Open_state : sig
  type t =
    | Managed of { initially_open : bool }
    | Controlled of bool
  [@@deriving equal, sexp_of]
end

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** Managed tooltips keep transient open state in Rust. Controlled tooltips
      request changes through [on_open_change] and follow the supplied Boolean.
      Delays are bounded to 0..60 seconds. Focus opens immediately; hover uses the
      show delay, with a shared per-window grace interval between tooltips.
      Default placement is Top/Center with a six-pixel gap. [disabled] suppresses
      the tooltip without disabling its anchor. [initially_open] is read only on
      mount; switching from Controlled to Managed retains current visibility.
      Escape and pointer-down on the anchor dismiss until the next hover/focus
      entry. [hoverable=false] disables pointer interaction with the content. *)
  val create
    :  label:string
    -> ?width:float
    -> ?placement:Placement.t
    -> ?open_state:Open_state.t
    -> ?disabled:bool
    -> ?hoverable:bool
    -> ?show_delay:Time_ns.Span.t
    -> ?hide_delay:Time_ns.Span.t
    -> ?skip_delay:Time_ns.Span.t
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val to_wire : Config.t -> Gpuio_protocol.Wire.Tooltip.t
  val placement : Config.t -> Placement.t
  val is_disabled : Config.t -> bool
end
