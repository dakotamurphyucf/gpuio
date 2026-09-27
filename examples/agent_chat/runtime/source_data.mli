open Core

(** In-memory sample sources. IDs describe fixture identity, never real paths. *)
val id : string -> Gpuio.Tree.Id.t

val initial : unit -> string Gpuio.Tree.t
val large : unit -> string Gpuio.Tree.t
val empty : unit -> string Gpuio.Tree.t

(** The notes branch deliberately fails its first attempt. No I/O occurs here. *)
val load
  :  attempt:int
  -> Gpuio.Tree_loading.Request.t
  -> string Gpuio.Tree_loading.Page.t Or_error.t

(** Recheck the proposal against current data and preferences before an atomic
    move. Preserve IDs and payloads; reject stale or incompletely loaded targets. *)
val approve
  :  'a Gpuio.Tree_loading.Snapshot.t
  -> state:Gpuio.Tree_state.t
  -> Gpuio.Tree_interaction.Move.t
  -> 'a Gpuio.Tree.t Or_error.t
