open Core

(** Internal runner adapter. All operations check the creating domain. A logical
    display cycle means native acceptance, not physical screen presentation. *)
type t

val create
  :  Gpuio_protocol.Window_id.t
  -> start:Time_ns.t
  -> theme:Gpuio.Theme.t
  -> unit Bonsai.Effect.t Gpuio.View.t Bonsai.Computation.t
  -> t

(** Refresh before delivering effects, so newly requested sleeps use this turn
    rather than the preceding frame's clock sample. *)
val advance_clock : t -> now:Time_ns.t -> unit

val cycle : t -> now:Time_ns.t -> unit Or_error.t
val next_message : t -> Gpuio_protocol.Wire.Message.t option
val submitted : t -> unit
val acknowledge : t -> revision:int64 -> unit Or_error.t
val dispatch : t -> Gpuio_protocol.Wire.Event.t -> unit
val schedule : t -> unit Bonsai.Effect.t -> unit
val set_theme : t -> Gpuio.Theme.t -> unit
val close : t -> unit
val revision : t -> int64
val cycles : t -> int
