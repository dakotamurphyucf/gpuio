(** Window-owned settings and explicitly simulated connection state. Native
    drafts live only while their page is mounted; closing or changing pages
    discards uncommitted drafts. Accepted values survive until window close. *)
type t

val create : unit -> t
val toggle : t -> unit
val annotation : t -> Gpuio.Color_value.Value.t Bonsai.Cont.t

val component
  :  t
  -> window:Gpuio_eio.App.Window.t
  -> results:Results.t
  -> on_generation:(Generation_settings.t -> unit)
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
