(** A window's confirmed concrete annotation color. Empty means use the current
    theme accent. Native picker previews do not change this value. Activate the
    computation only while its page is present so deactivation cancels drafts. *)
type t

val create : unit -> t
val value : t -> Gpuio.Color_value.Value.t Bonsai.Cont.t

val component
  :  t
  -> window:Gpuio_eio.App.Window.t
  -> is_current:(unit -> bool) Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
