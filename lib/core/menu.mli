(** An immutable, labeled menu of command references. Command labels, checked
    state and availability come from the enclosing command registry. *)
type t [@@deriving equal, sexp_of]

module Item_path : sig
  type t [@@deriving equal, sexp_of]

  (** Zero-based root menu index followed by one or more item indices. For
      example, [0; 2; 1] addresses the second item of the third item's submenu
      in the first menu. Construction checks structural bounds; the receiving
      view checks that the path exists. Paths identify positions, not commands. *)
  val of_list : int list -> t Core.Or_error.t

  val to_list : t -> int list
end

module Item : sig
  type menu := t

  type t =
    | Command of Command.Id.t
    | Separator
    | Submenu of menu
    | Label of string
  [@@deriving equal, sexp_of]
end

(** Nonblank UTF-8 labels without NUL, at most 4096 bytes each, including
    [Item.Label] section labels. Labels are noninteractive text rows: keyboard
    navigation/typeahead skip them and they never resolve a command. They are
    supported in drawn button/context menus and [View.menu_bar ~platform:false].
    Platform context popups show labels as disabled, non-actionable native rows.
    Platform menu bars reject labels rather than converting them into commands.
    A menu is limited to
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
    | Editor_context
    | Platform_context
  [@@deriving equal, sexp_of]

  val command_ids : t -> Command.Id.t list
  val item_paths : t list -> (Item_path.t * Item.t) list
  val validate_collection : t list -> unit Core.Or_error.t
  val validate_platform_collection : t list -> unit Core.Or_error.t
  val to_wire : t -> Gpuio_protocol.Wire.Menu_definition.t
end
