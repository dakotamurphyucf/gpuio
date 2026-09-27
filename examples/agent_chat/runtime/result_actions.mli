(** Window-owned finding inspection. Captured targets contain a source membership,
    query generation and dialog identity, never a row payload or source snapshot. *)
type t

val create : unit -> t

val request
  :  t
  -> generation:int64
  -> Gpuio.Table_data.Row_ref.t Gpuio.Table.Request.t
  -> unit Bonsai.Effect.t

(** Right-click/Shift-F10 and activation open an ordinary accessible dialog.
    Reveal selects the result cell and returns focus to the table. Both display
    and invocation validate the target against the current pager snapshot. *)
val view
  :  t
  -> snapshot:('query, 'row) Gpuio.Table_paging.Snapshot.t Bonsai.Cont.t
  -> current:(unit -> ('query, 'row) Gpuio.Table_paging.Snapshot.t)
  -> output:'row Gpuio_bonsai.Table.Output.t Core.Or_error.t Bonsai.Cont.t
  -> describe:('row -> string)
  -> result_column:Gpuio.Table_column.Id.t
  -> dark:bool Bonsai.Cont.t
  -> Bonsai.Cont.graph
  -> Gpuio_bonsai.View.t Bonsai.Cont.t
