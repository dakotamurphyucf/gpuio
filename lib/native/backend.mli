(** Native archive provider. Implementations initialize their statically composed
    component registry before a transport is created. Called once per process
    on the main thread; failure prevents native application startup. *)
val initialize : unit -> unit
