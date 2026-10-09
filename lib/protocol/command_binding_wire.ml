open Core
module C = Command_wire

exception Invalid_wire_binding

let max_targets = 64
let max_observers = 64
let max_config_bytes = 32768
let max_observation_bytes = 262144
let max_strokes = 8

let valid_text text max_bytes =
  (not (String.is_empty (String.strip text)))
  && String.length text <= max_bytes
  && Stdlib.String.is_valid_utf_8 text
  && not (String.contains text '\000')
;;

let read_list buffer ~pos_ref ~max ~read =
  let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
  if count < 0 || count > max then raise Invalid_wire_binding;
  if count > Bigstring.length buffer - !pos_ref then raise Bin_prot.Common.Buffer_short;
  let rec loop count reversed =
    if count = 0
    then List.rev reversed
    else loop (count - 1) (read buffer ~pos_ref :: reversed)
  in
  loop count []
;;

let read_text max buffer ~pos_ref =
  let start = !pos_ref in
  let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
  if count < 0 || count > max then raise Invalid_wire_binding;
  if count > Bigstring.length buffer - !pos_ref then raise Bin_prot.Common.Buffer_short;
  pos_ref := start;
  Bin_prot.Read.bin_read_string buffer ~pos_ref
;;

module Context = struct
  type t =
    | Focused
    | Here
    | Editor of Window_id.t * Node_id.t
    | Native_context of string
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Native_context text -> valid_text text 1024
    | Focused | Here | Editor _ -> true
  ;;

  let bin_read_t buffer ~pos_ref =
    match Bin_prot.Read.bin_read_int_8bit buffer ~pos_ref with
    | 0 -> Focused
    | 1 -> Here
    | 2 ->
      let window = Window_id.bin_read_t buffer ~pos_ref in
      let node = Node_id.bin_read_t buffer ~pos_ref in
      Editor (window, node)
    | 3 -> Native_context (read_text 1024 buffer ~pos_ref)
    | _ -> raise Invalid_wire_binding
  ;;
end

module Target = struct
  type t =
    | Command of string
    | Native_action of C.Native_command.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Command id -> valid_text id 256
    | Native_action _ -> true
  ;;

  let bin_read_t buffer ~pos_ref =
    match Bin_prot.Read.bin_read_int_8bit buffer ~pos_ref with
    | 0 -> Command (read_text 256 buffer ~pos_ref)
    | 1 -> Native_action (C.Native_command.bin_read_t buffer ~pos_ref)
    | _ -> raise Invalid_wire_binding
  ;;
end

module Targets = struct
  type t = Target.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t = read_list ~max:max_targets ~read:Target.bin_read_t
end

module Config = struct
  type t =
    { context : Context.t
    ; targets : Targets.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Context.valid t.context
    && (not (List.is_empty t.targets))
    && List.length t.targets <= max_targets
    && List.for_all t.targets ~f:Target.valid
    && List.for_alli t.targets ~f:(fun i target ->
      not (List.mem (List.take t.targets i) target ~equal:Target.equal))
    && List.for_all t.targets ~f:(fun target ->
      match t.context, target with
      | Here, Native_action _ | Native_context _, Command _ -> false
      | Focused, _ | Editor _, _ | Here, Command _ | Native_context _, Native_action _ ->
        true)
    && bin_size_t t <= max_config_bytes
  ;;

  let bin_read_t buffer ~pos_ref =
    let start = !pos_ref in
    let t = bin_read_t buffer ~pos_ref in
    if !pos_ref - start > max_config_bytes || not (valid t)
    then raise Invalid_wire_binding;
    t
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module Suppression = struct
  type t =
    | Disabled
    | Scope_blocked
    | Native_unavailable
    | Composition
    | Text_input
    | Native_navigation
    | Conflict of string
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Conflict id -> valid_text id 256
    | Disabled
    | Scope_blocked
    | Native_unavailable
    | Composition
    | Text_input
    | Native_navigation -> true
  ;;

  let bin_read_t buffer ~pos_ref =
    match Bin_prot.Read.bin_read_int_8bit buffer ~pos_ref with
    | 0 -> Disabled
    | 1 -> Scope_blocked
    | 2 -> Native_unavailable
    | 3 -> Composition
    | 4 -> Text_input
    | 5 -> Native_navigation
    | 6 -> Conflict (read_text 256 buffer ~pos_ref)
    | _ -> raise Invalid_wire_binding
  ;;
end

module Disposition = struct
  type t =
    | Declared
    | Override
    | Native_first
    | Widget
    | Unavailable of Suppression.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Unavailable reason -> Suppression.valid reason
    | Declared | Override | Native_first | Widget -> true
  ;;
end

let valid_shortcut (t : C.Shortcut.t) =
  let named =
    List.mem
      [ "enter"
      ; "escape"
      ; "tab"
      ; "space"
      ; "backspace"
      ; "delete"
      ; "insert"
      ; "home"
      ; "end"
      ; "pageup"
      ; "pagedown"
      ; "left"
      ; "right"
      ; "up"
      ; "down"
      ]
      t.key
      ~equal:String.equal
  in
  let function_key =
    List.exists
      (List.init 24 ~f:(fun i -> "f" ^ Int.to_string (i + 1)))
      ~f:(String.equal t.key)
  in
  let scalar =
    Stdlib.String.is_valid_utf_8 t.key
    && String.length t.key > 0
    && String.count t.key ~f:(fun byte -> Char.to_int byte land 0xc0 <> 0x80) = 1
    && (not
          (String.length t.key = 2
           && Char.to_int t.key.[0] = 0xc2
           && Char.to_int t.key.[1] <= 0x9f))
    && String.for_all t.key ~f:(fun byte ->
      Char.to_int byte >= 0x21 && Char.to_int byte <> 0x7f)
  in
  let rank = function
    | C.Shortcut_modifier.Primary -> 0
    | Control -> 1
    | Alt -> 2
    | Shift -> 3
    | Super -> 4
  in
  (named || function_key || scalar)
  && String.equal t.key (String.lowercase t.key)
  && List.length t.modifiers <= 5
  && List.is_sorted_strictly (List.map t.modifiers ~f:rank) ~compare:Int.compare
;;

module Candidate = struct
  type t =
    { shortcut : C.Shortcut.t
    ; disposition : Disposition.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t = valid_shortcut t.shortcut && Disposition.valid t.disposition

  let bin_read_t buffer ~pos_ref =
    let key = read_text 256 buffer ~pos_ref in
    let modifiers =
      read_list buffer ~pos_ref ~max:5 ~read:C.Shortcut_modifier.bin_read_t
    in
    let priority = C.Shortcut_priority.bin_read_t buffer ~pos_ref in
    let text_input = C.Shortcut_text_input.bin_read_t buffer ~pos_ref in
    let during_composition = Bin_prot.Read.bin_read_bool buffer ~pos_ref in
    let disposition = Disposition.bin_read_t buffer ~pos_ref in
    { shortcut = { C.Shortcut.key; modifiers; priority; text_input; during_composition }
    ; disposition
    }
  ;;
end

module Candidates = struct
  type t = Candidate.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t = read_list ~max:4 ~read:Candidate.bin_read_t
end

module Stroke = struct
  (** Physical modifier bits: Control=1, Alt=2, Shift=4, platform=8, Function=16.
      Display data only: may contain native keys outside the input chord domain. *)
  type t =
    { key : string
    ; modifiers : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let rec no_controls offset =
      if offset = String.length t.key
      then true
      else (
        let decoded = Stdlib.String.get_utf_8_uchar t.key offset in
        let code = Stdlib.Uchar.to_int (Stdlib.Uchar.utf_decode_uchar decoded) in
        code > 31
        && (not (code >= 127 && code <= 159))
        && no_controls (offset + Stdlib.Uchar.utf_decode_length decoded))
    in
    valid_text t.key 256
    && no_controls 0
    && Int64.(t.modifiers >= 0L && t.modifiers <= 31L)
  ;;

  let bin_read_t buffer ~pos_ref =
    let key = read_text 256 buffer ~pos_ref in
    let modifiers = Bin_prot.Read.bin_read_int64 buffer ~pos_ref in
    { key; modifiers }
  ;;
end

module Strokes = struct
  type t = Stroke.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t = read_list ~max:max_strokes ~read:Stroke.bin_read_t
end

module Unsupported = struct
  type t =
    | Sequence_too_long
    | Invalid_stroke
  [@@deriving bin_io, equal, sexp_of]
end

module Entry = struct
  type t =
    | Missing_command
    | Registry of
        { enabled : bool
        ; candidates : Candidates.t
        }
    | Native_unbound
    | Native_binding of
        { strokes : Strokes.t
        ; disposition : Disposition.t
        }
    | Native_unsupported of Unsupported.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Missing_command | Native_unbound | Native_unsupported _ -> true
    | Registry { enabled = _; candidates } ->
      List.length candidates <= 4 && List.for_all candidates ~f:Candidate.valid
    | Native_binding { strokes; disposition } ->
      (not (List.is_empty strokes))
      && List.length strokes <= max_strokes
      && List.for_all strokes ~f:Stroke.valid
      && Disposition.valid disposition
  ;;
end

module Entries = struct
  type t = Entry.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t = read_list ~max:max_targets ~read:Entry.bin_read_t
end

module State = struct
  type t =
    | Ready of Entries.t
    | Suspended
    | Context_gone
    | Invalid_context
    | Epoch_exhausted
    | Capacity
  [@@deriving bin_io, equal, sexp_of]
end

module Observation = struct
  type t =
    { epoch : int64
    ; state : State.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.epoch > 0L)
    && (match t.state with
        | Ready entries ->
          List.length entries <= max_targets && List.for_all entries ~f:Entry.valid
        | Suspended | Context_gone | Invalid_context | Epoch_exhausted | Capacity -> true)
    && bin_size_t t <= max_observation_bytes
  ;;

  let valid_for t (config : Config.t) =
    valid t
    && Config.valid config
    &&
    let focused = Context.equal config.context Focused in
    let declared disposition = Disposition.equal disposition Declared in
    match t.state with
    | Suspended | Epoch_exhausted | Capacity -> true
    | Context_gone ->
      (match config.context with
       | Editor _ -> true
       | Focused | Here | Native_context _ -> false)
    | Invalid_context ->
      (match config.context with
       | Native_context _ -> true
       | Focused | Here | Editor _ -> false)
    | Ready entries ->
      List.length entries = List.length config.targets
      && List.for_all2_exn entries config.targets ~f:(fun entry target ->
        match target, entry with
        | Target.Command _, Entry.Missing_command
        | Native_action _, (Native_unbound | Native_unsupported _) -> true
        | Command command, Registry { enabled; candidates } ->
          List.for_all candidates ~f:(fun candidate ->
            if not focused
            then declared candidate.disposition
            else (
              match candidate.disposition with
              | Declared | Widget -> false
              | Override ->
                enabled && C.Shortcut_priority.equal candidate.shortcut.priority Override
              | Native_first ->
                enabled
                && C.Shortcut_priority.equal candidate.shortcut.priority Native_first
              | Unavailable Disabled -> not enabled
              | Unavailable (Conflict id) -> enabled && not (String.equal id command)
              | Unavailable
                  ( Scope_blocked
                  | Native_unavailable
                  | Composition
                  | Text_input
                  | Native_navigation ) -> enabled))
        | Native_action _, Native_binding { strokes = _; disposition } ->
          if not focused
          then declared disposition
          else (
            match disposition with
            | Widget | Unavailable _ -> true
            | Declared | Override | Native_first -> false)
        | Command _, (Native_unbound | Native_binding _ | Native_unsupported _)
        | Native_action _, (Missing_command | Registry _) -> false)
  ;;

  let bin_read_t buffer ~pos_ref =
    let start = !pos_ref in
    let t = bin_read_t buffer ~pos_ref in
    if !pos_ref - start > max_observation_bytes || not (valid t)
    then raise Invalid_wire_binding;
    t
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end
