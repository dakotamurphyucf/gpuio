(** Per-window artifact navigation and visibility. Closing removes native child
    resources; the window-owned review model remains available on reopen. *)
type t

val create : unit -> t
val toggle : t -> unit

val component
  :  t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
