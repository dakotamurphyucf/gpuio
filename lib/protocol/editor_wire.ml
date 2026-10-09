open Core

module Selection = struct
  type t =
    { anchor : int64
    ; head : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Privacy = struct
  type t =
    | Plain
    | Password_hidden
    | Password_revealed
  [@@deriving bin_io, equal, sexp_of]
end

module Config = struct
  type t =
    { label : string
    ; placeholder : string
    ; read_only : bool
    ; disabled : bool
    ; submit_on_enter : bool
    ; auto_focus : bool
    ; min_rows : int64
    ; max_rows : int64
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Snapshot = struct
  type t =
    { revision : int64
    ; text : string
    ; selection : Selection.t
    ; composition : Selection.t option
    ; focused : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Selection_policy = struct
  type t =
    | Start
    | End
    | Preserve
    | Select of Selection.t
  [@@deriving bin_io, equal, sexp_of]
end

module Undo_policy = struct
  type t =
    | Record
    | Reset
  [@@deriving bin_io, equal, sexp_of]
end

module Command = struct
  type t =
    | Replace of string * Selection_policy.t * Undo_policy.t * int64 option
    | Select of Selection.t
    | Focus
    | Undo
    | Redo
    | Submit
    | Read_snapshot
    | Read_content_hint_status
    | Read_viewport
    | Scroll_viewport of Editor_viewport_wire.Offset.t
    | Search of Editor_search_wire.Command.t
    | Read_range_bounds of int64 * Selection.t
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_editor
    | Stale_revision
    | Composing
    | Invalid_selection
    | Limit_exceeded
    | Busy
    | Native_failure
    | Invalid_text
    | Focus_blocked
    | Search_unavailable
    | Stale_search
    | Not_editable
  [@@deriving bin_io, equal, sexp_of]
end

module Result = struct
  type t =
    | Applied of Snapshot.t
    | Failed of Error.t
    | Content_hint_status of Input_content_hint_status_wire.t
    | Viewport of Editor_viewport_wire.t option
    | Viewport_scroll_accepted
    | Search_observed of Editor_search_wire.Snapshot.t
    | Search_replaced of Snapshot.t * Editor_search_wire.Snapshot.t * int64
    | Range_bounds of Editor_geometry_wire.t option
  [@@deriving bin_io, equal, sexp_of]
end

module Event_kind = struct
  type t =
    | Changed
    | Submitted
  [@@deriving bin_io, equal, sexp_of]
end
