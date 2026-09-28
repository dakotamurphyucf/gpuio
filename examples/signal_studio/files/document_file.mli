open Core
module Workspace = Signal_studio_model.Workspace

(** Bounded reads (at most 16 KiB plus one byte to detect excess), followed by
    full workspace validation. Expected filesystem/format failures return Error;
    Eio cancellation propagates. No native handles or model state are mutated. *)
val load : _ Eio.Path.t -> Workspace.t Or_error.t

(** Save one immutable workspace snapshot. Creates a private exclusive temporary
    sibling, writes and syncs it, then atomically replaces the selected destination.
    The containing directory is held open throughout. Failure/cancellation removes
    the temporary file; the old destination is untouched before rename. This does
    not preserve existing permissions or promise directory crash durability.
    Symlinks at the destination are replaced, not followed. *)
val save : _ Eio.Path.t -> random:_ Eio.Flow.source -> Workspace.t -> unit Or_error.t
