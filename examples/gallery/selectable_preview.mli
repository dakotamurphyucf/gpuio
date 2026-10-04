(** Window-owned searchable data; producers outlive transient rows and close with
    the supplied scope. Construct outside Incremental graph evaluation. *)
type t

val create : scope:Gpuio_eio.Scope.t -> clock:_ Eio.Time.Mono.t -> t Core.Or_error.t

val component
  :  t
  -> Gpuio_eio.App.Window.t
  -> Palette.t Bonsai.Cont.t
  -> Gpuio.Scrollbar.t option Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
