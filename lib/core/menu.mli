(** An immutable, labeled menu of command references. Command labels, checked
    state and availability come from the enclosing command registry. *)
type t [@@deriving equal, sexp_of]

module Item : sig
  type menu := t

  type t =
    | Command of Command.Id.t
    | Separator
    | Submenu of menu
  [@@deriving equal, sexp_of]
end

(** Nonblank UTF-8 label without NUL, at most 4096 bytes. A menu is limited to
    eight nested levels, 1024 items and 256 KiB of labels/command IDs. Disabled
    submenus cannot open. Empty menus are permitted. *)
val create : label:string -> ?disabled:bool -> Item.t list -> t Core.Or_error.t

val label : t -> string

(** Popup, row and empty-state styles/geometry use the same vocabulary as choices. *)
module Appearance = Choice.Appearance

module Expert : sig
  type presentation =
    | Button
    | Context
    | Bar
    | Platform_bar
  [@@deriving equal, sexp_of]

  val command_ids : t -> Command.Id.t list
  val validate_collection : t list -> unit Core.Or_error.t
  val to_wire : t -> Gpuio_protocol.Wire.Menu_definition.t
end
