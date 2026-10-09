open Core
module W = Gpuio_protocol.Editor_search_wire

module Query = struct
  type t = string [@@deriving equal, sexp_of]

  let create query =
    if W.valid_query query
    then Ok query
    else Or_error.error_string "search query must be at most 2048 UTF-8 bytes without NUL"
  ;;

  let empty = ""
  let to_string t = t
end

module Case = W.Case
module Mode = W.Mode

module Stamp = struct
  type t =
    { window : Gpuio_protocol.Window_id.t
    ; node : Gpuio_protocol.Node_id.t
    ; value : W.Stamp.t
    }
  [@@deriving equal, sexp_of]
end

module Occurrence = struct
  type t = W.Occurrence.t [@@deriving equal, sexp_of]

  let index t = Int64.to_int_exn t.W.Occurrence.index
  let byte_start t = Int64.to_int_exn t.W.Occurrence.byte_start
  let byte_end t = Int64.to_int_exn t.W.Occurrence.byte_end
end

module Snapshot = struct
  type t =
    { stamp : Stamp.t
    ; value : W.Snapshot.t
    }
  [@@deriving equal, sexp_of]

  let stamp t = t.stamp
  let mode t = t.value.mode
  let query t = t.value.query
  let case t = t.value.case
  let match_count t = Int64.to_int_exn t.value.match_count
  let current t = t.value.current
  let can_replace t = t.value.can_replace
  let activation_revision t = t.value.activation_revision
end

module Command = struct
  type t =
    | Read
    | Close_and_focus of Snapshot.t
    (** Close this particular opening and restore focus atomically. Checks the
        observed owner and activation, so an older bar cannot close a reopened
        search. Text/query/navigation changes within that opening are allowed.
        Disabled or hidden targets close without restoring focus. Composed or
        already closed targets fail without focus changes. Ordinary [Close]
        remains metadata-only. *)
    | Open of { replace : bool }
    | Close
    | Set_query of
        { query : Query.t
        ; case : Case.t
        }
    | Set_query_text of Query.t
    (** Change query text while preserving the current native case policy. *)
    | Set_case of Case.t
    (** Change case policy while preserving the current native query text. *)
    | Toggle_case
    (** Toggle the current native policy; rapid activations do not reuse a stale
        rendered checkbox value. *)
    | Next
    | Previous
    | Replace_current of
        { if_stamp : Stamp.t
        ; replacement : string
        }
    | Replace_all of
        { if_stamp : Stamp.t
        ; replacement : string
        }
  [@@deriving equal, sexp_of]
end

module Response = struct
  type t =
    | Observed of Snapshot.t
    | Replaced of
        { snapshot : Snapshot.t
        ; count : int
        }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let snapshot_of_wire ~window ~node (value : W.Snapshot.t) =
    if W.Snapshot.is_valid value
    then Ok { Snapshot.stamp = { Stamp.window; node; value = value.stamp }; value }
    else Or_error.error_string "invalid native search observation"
  ;;

  let stamp_to_wire t = t.Stamp.value
  let stamp_window t = t.Stamp.window
  let stamp_node t = t.Stamp.node

  let command_to_wire : Command.t -> W.Command.t = function
    | Read -> Read
    | Close_and_focus snapshot -> Close_and_focus (Snapshot.activation_revision snapshot)
    | Open { replace } -> Open replace
    | Close -> Close
    | Set_query { query; case } -> Set_query (query, case)
    | Set_query_text query -> Set_query_text query
    | Set_case case -> Set_case case
    | Toggle_case -> Toggle_case
    | Next -> Next
    | Previous -> Previous
    | Replace_current { if_stamp; replacement } ->
      Replace_current (if_stamp.Stamp.value, replacement)
    | Replace_all { if_stamp; replacement } ->
      Replace_all (if_stamp.Stamp.value, replacement)
  ;;

  let expected_stamp : Command.t -> Stamp.t option = function
    | Replace_current { if_stamp; _ } | Replace_all { if_stamp; _ } -> Some if_stamp
    | Close_and_focus snapshot -> Some (Snapshot.stamp snapshot)
    | Read
    | Open _
    | Close
    | Set_query _
    | Set_query_text _
    | Set_case _
    | Toggle_case
    | Next
    | Previous -> None
  ;;
end
