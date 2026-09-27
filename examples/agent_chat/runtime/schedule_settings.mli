(** Accepted civil selections belong to the window. Activate this computation
    only while its settings page is present; picker deactivation cancels drafts.
    [is_current] fences writes from retired outer settings pages. *)
type t

val create : unit -> t

val component
  :  t
  -> window:Gpuio_eio.App.Window.t
  -> is_current:(unit -> bool) Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
