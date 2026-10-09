open Core

module State : sig
  type t =
    | Pending
    | Collapsed
    | Source_view
    | Rich of { clamped : bool }
  [@@deriving equal, sexp_of]
end

module Event : sig
  (** Describes the installed picture, which can precede a newer parse in the
      same source generation. Before the first picture, [Pending] names the
      requested snapshot. [Source_view] includes explicit source mode and rich
      rendering fallback; it does not claim that the rich preview fits.
      Notifications are queued after native painting, never synchronous effects
      from layout. Identical state/provenance/config observations are suppressed. *)
  type t = private
    { source_revision : int64
    ; source_generation : int64
    ; state : State.t
    }
  [@@deriving equal, sexp_of]
end

module Expert : sig
  val event_of_wire : Gpuio_protocol.Document_preview_wire.Event.t -> Event.t Or_error.t
end
