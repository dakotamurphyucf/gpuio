(** Owns Eio services, native resources, model actions and window lifetimes.
    Component wires observable state into the stateless Ui.view. *)
val run : unit -> unit
