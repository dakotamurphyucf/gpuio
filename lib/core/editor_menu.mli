(** Labels for the native edit menu of one single-line input or text area.
    Actions are fixed: Cut, Copy, Paste, a separator and Select all. Availability
    comes from the current native editor; no editing callback enters OCaml. *)
type t [@@deriving equal, sexp_of]

(** Each label is nonblank UTF-8 without NUL, at most 4096 bytes. *)
val create
  :  ?enabled:bool
  -> ?label:string
  -> ?cut:string
  -> ?copy:string
  -> ?paste:string
  -> ?select_all:string
  -> unit
  -> t Core.Or_error.t

val default : t

module Expert : sig
  val menu : t -> Menu.t
  val commands : t -> 'action Command.Registry.t
end
