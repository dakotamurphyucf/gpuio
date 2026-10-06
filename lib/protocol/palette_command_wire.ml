open Core

module Command = struct
  type t =
    | Read_snapshot
    | Focus
    | Set_query of string
    | Highlight of string option
    | Set_loading of bool
    | Publish_results of Palette_results_wire.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Publish_results results -> Palette_results_wire.valid results
    | Read_snapshot | Focus | Highlight None | Set_loading _ -> true
    | Set_query query ->
      Palette_state_wire.valid
        { sequence = 1L
        ; query_revision = 1L
        ; query
        ; composing = false
        ; selected = None
        ; matched_count = 0
        ; loading = false
        }
    | Highlight (Some id) ->
      Palette_state_wire.valid
        { sequence = 1L
        ; query_revision = 1L
        ; query = ""
        ; composing = false
        ; selected = Some id
        ; matched_count = 1
        ; loading = false
        }
  ;;
end

module Error = struct
  type t =
    | Not_mounted
    | Closed
    | Stale_palette
    | Query_changed
    | Composing
    | Unavailable
    | Invalid_query
    | Busy
    | Native_failure
    | Invalid_results
  [@@deriving bin_io, equal, sexp_of]
end

module Response = struct
  type t =
    | Applied of Palette_state_wire.t
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Applied snapshot -> Palette_state_wire.valid snapshot
    | Failed _ -> true
  ;;
end
