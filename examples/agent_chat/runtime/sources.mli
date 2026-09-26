open Core

(** Window-owned sample loader and approval state, independent of virtual rows.
    Create in the window factory, before evaluating its Bonsai graph. [sleep] and
    [build_large] run in scoped Eio tasks; the latter should use a worker domain. *)
type t

val create
  :  scope:Gpuio_eio.Scope.t
  -> sleep:(float -> unit)
  -> build_large:(unit -> string Gpuio.Tree.t)
  -> t Or_error.t

val component
  :  t
  -> active:bool Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
