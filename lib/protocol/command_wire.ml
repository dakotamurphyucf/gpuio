open Core

module Shortcut_modifier = struct
  type t =
    | Primary
    | Control
    | Alt
    | Shift
    | Super
  [@@deriving bin_io, equal, sexp_of]
end

module Shortcut_priority = struct
  type t =
    | Native_first
    | Override
  [@@deriving bin_io, equal, sexp_of]
end

module Shortcut_text_input = struct
  type t =
    | Modified_only
    | Always
    | Never
  [@@deriving bin_io, equal, sexp_of]
end

module Shortcut = struct
  type t =
    { key : string
    ; modifiers : Shortcut_modifier.t list
    ; priority : Shortcut_priority.t
    ; text_input : Shortcut_text_input.t
    ; during_composition : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Native_command = struct
  type t =
    | Copy
    | Cut
    | Paste
    | Select_all
    | Undo
    | Redo
  [@@deriving bin_io, equal, sexp_of]
end

module Command_target = struct
  type t =
    | Callback
    | Native of Native_command.t
  [@@deriving bin_io, equal, sexp_of]
end

module Command = struct
  type t =
    { id : string
    ; generation : int64
    ; label : string
    ; enabled : bool
    ; checked : bool option
    ; shortcuts : Shortcut.t list
    ; target : Command_target.t
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Command_source = struct
  type t =
    | Button of Node_id.t
    | Shortcut
    | Menu of Node_id.t
    | Palette of Node_id.t
  [@@deriving bin_io, equal, sexp_of]
end
