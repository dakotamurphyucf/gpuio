open Core

(** Export at most 64 KiB to the user-selected destination. Writes/syncs an
    exclusive private sibling and atomically renames it; failure or cancellation
    cleans up the sibling. Replaces destination symlinks, does not preserve old
    permissions, and does not promise directory crash durability. Eio cancellation
    propagates. Expected filesystem failures return errors. *)
val save : _ Eio.Path.t -> random:_ Eio.Flow.source -> string -> unit Or_error.t
