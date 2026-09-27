(** Per-window artifact navigation and visibility. Closing unmounts native views;
    the review model and one lazily registered diagram survive until window close.
    Visit history is bounded to 32 entries, with at most four visible breadcrumbs. *)
type t

val create : unit -> t
val toggle : t -> unit

val component
  :  t
  -> app:Gpuio_eio.App.t
  -> window:Gpuio_eio.App.Window.t
  -> sources:Sources.t
  -> results:Results.t
  -> annotation:Gpuio.Color_value.Value.t Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
