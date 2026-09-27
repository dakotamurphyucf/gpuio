open Core

(** Shared ownership vocabulary: Managed keeps transient visibility in Rust;
    Controlled follows the application's accepted Boolean and emits requests. *)
module Open_state = Tooltip.Open_state

module Config : sig
  type t [@@deriving equal, sexp_of]

  (** An interactive, nonmodal preview. Hover opens after [show_delay] (600 ms by
      default); leaving both trigger and panel closes after [hide_delay] (300 ms).
      Focus within either opens immediately and keeps it open. Delays are bounded
      to 0..60 seconds. Cards do not share the tooltip grace interval.

      Defaults: width 320 logical pixels, Top/Center placement with a six-pixel
      gap, Managed initially closed, enabled. Labels and widths follow Overlay
      validation. Disabled hides the card without disabling its trigger.
      Initial managed visibility is read only on mount; Controlled-to-Managed
      retains the accepted visibility. No permanent idle polling is used. *)
  val create
    :  label:string
    -> ?width:float
    -> ?placement:Placement.t
    -> ?open_state:Open_state.t
    -> ?disabled:bool
    -> ?show_delay:Time_ns.Span.t
    -> ?hide_delay:Time_ns.Span.t
    -> unit
    -> t Or_error.t
end

module Expert : sig
  val tooltip : Config.t -> Tooltip.Config.t
end
