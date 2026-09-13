open Core

module Id : sig
  type t [@@deriving equal, compare, sexp_of]

  (** 1..256 UTF-8 bytes without NUL, with at least one byte outside Core's
      ASCII whitespace set. IDs are preserved exactly, without normalization. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Native : sig
  type t =
    | Copy
    | Cut
    | Paste
    | Select_all
    | Undo
    | Redo
  [@@deriving equal, sexp_of]
end

type 'action t

val create
  :  id:Id.t
  -> label:string
  -> ?enabled:bool
  -> ?checked:bool
  -> ?shortcuts:Shortcut.t list
  -> on_invoke:(unit -> 'action)
  -> unit
  -> 'action t Or_error.t

(** Edit actions execute against the focused native editor, without an OCaml
    editing round trip. Their availability also depends on native edit state. *)
val native
  :  id:Id.t
  -> label:string
  -> ?enabled:bool
  -> ?checked:bool
  -> ?shortcuts:Shortcut.t list
  -> Native.t
  -> 'action t Or_error.t

val id : _ t -> Id.t
val label : _ t -> string
val is_enabled : _ t -> bool

module Registry : sig
  type 'action command := 'action t
  type 'action t

  (** At most 1024 commands, four shortcuts per command and 256 KiB aggregate
      text. IDs are unique within this registry; nested scopes may shadow them.
      Declaration order is preserved for presentation and shortcut precedence. *)
  val create : 'action command list -> 'action t Or_error.t

  val to_list : 'action t -> 'action command list
  val find : 'action t -> Id.t -> 'action command option
end

module Expert : sig
  val to_wire : _ t -> generation:int64 -> Gpuio_protocol.Wire.Command.t
  val invoke : 'action t -> 'action option
end
