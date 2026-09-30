open Core

(** One example editor placement lifetime, separate from its retained controller.
    Allocate a new visit for each managed-row activation, activate/deactivate it
    with that row's acknowledged lifecycle, and guard native callbacks with the
    row lifetime too. This mutable owner stays on the UI domain. *)
type t

val create : unit -> t

(** Activation of a retired visit is a programming error; allocate a fresh one. *)
val activate : t -> unit

val deactivate : t -> unit
val is_active : t -> bool

(** Accept a nonnegative revision only on this active visit, rejecting older
    native observations/replies. Equal revisions are harmless duplicates and
    return false. A fresh visit accepts revision zero independently of old visits. *)
val observe : t -> revision:int64 -> bool
