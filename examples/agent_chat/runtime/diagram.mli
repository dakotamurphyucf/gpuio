(** One lazily registered canvas per window. The window scope owns the scene;
    hidden native views may unmount without discarding the application model.
    There are no OCaml callbacks for intermediate native paint/drag frames. *)
type t

val create : unit -> t

val component
  :  t
  -> app:Gpuio_eio.App.t
  -> window:Gpuio_eio.App.Window.t
  -> active:bool Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> annotation:Gpuio.Color_value.Value.t Bonsai.Cont.t
  -> on_open:(Run_diagram.Stage.t -> unit Bonsai.Effect.t)
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
