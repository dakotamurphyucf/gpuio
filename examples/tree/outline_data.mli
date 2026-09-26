open Core

val id : string -> Gpuio.Tree.Id.t
val initial : unit -> string Gpuio.Tree.t

(** Example application policy for a fully loaded outline. Revalidate the proposal
    against the latest snapshot and preferences, then return an atomic replacement.
    Reordering preserves IDs, incarnations and application payloads. A stale or
    ineligible proposal, or insertion into an incompletely loaded sibling list,
    returns Error without changing the input. No filesystem I/O is performed. *)
val approve
  :  'data Gpuio.Tree_loading.Snapshot.t
  -> state:Gpuio.Tree_state.t
  -> Gpuio.Tree_interaction.Move.t
  -> 'data Gpuio.Tree.t Or_error.t
