open Core

(** One window's result query, native column preferences and data pager. Create
    outside graph evaluation. [build] executes in a scoped Eio task and should
    use a worker domain for large complete-query transformations. *)
type t

val create
  :  scope:Gpuio_eio.Scope.t
  -> sleep:(float -> unit)
  -> build:
       (Result_data.Row.t Gpuio.Table_data.t
        -> Result_data.Query.t
        -> Result_data.Row.t Gpuio.Table_data.t)
  -> t Or_error.t

val component
  :  t
  -> active:bool Bonsai.Cont.t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
