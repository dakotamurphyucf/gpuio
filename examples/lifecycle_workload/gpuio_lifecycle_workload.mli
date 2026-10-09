(** Shared acknowledged resource lifecycle workload. Entity auditing requires the
    dedicated statically linked audit backend. *)
val run
  :  smoke:bool
  -> background:bool
  -> native_entities:bool
  -> metal_memory:bool
  -> presentation:bool
  -> unit
