(** Run a UI effect from an Eio task. Enqueue first, then await its result; do not
    call this blocking adapter inside a Bonsai effect or during view construction. *)
val perform : Gpuio_eio.Scope.t -> 'a Bonsai.Effect.t -> 'a
