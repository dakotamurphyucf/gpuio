module Ui_command = Command
open Core
module W = Gpuio_protocol.Command_binding_wire
module C = Gpuio_protocol.Command_wire

module Context = struct
  type t = W.Context.t [@@deriving equal, sexp_of]

  let focused = W.Context.Focused
  let here = W.Context.Here

  let editor snapshot =
    W.Context.Editor (Text_input.Expert.window snapshot, Text_input.Expert.node snapshot)
  ;;

  let native_context text =
    let context = W.Context.Native_context text in
    if W.Context.valid context
    then Ok context
    else
      Or_error.error_string
        "native key context must be nonblank UTF-8 without NUL, at most 1024 bytes"
  ;;
end

module Target = struct
  type t =
    | Command of Ui_command.Id.t
    | Native_action of Ui_command.Native.t
  [@@deriving equal, sexp_of]

  let to_wire = function
    | Command id -> W.Target.Command (Ui_command.Id.to_string id)
    | Native_action action ->
      W.Target.Native_action
        (match action with
         | Copy -> C.Native_command.Copy
         | Cut -> Cut
         | Paste -> Paste
         | Select_all -> Select_all
         | Undo -> Undo
         | Redo -> Redo)
  ;;
end

module Config = struct
  type t =
    { context : Context.t
    ; targets : Target.t list
    }
  [@@deriving equal, sexp_of]

  let to_wire t =
    { W.Config.context = t.context; targets = List.map t.targets ~f:Target.to_wire }
  ;;

  let create ?(context = Context.focused) targets =
    let t = { context; targets } in
    if W.Config.valid (to_wire t)
    then Ok t
    else
      Or_error.error_string
        "binding query needs 1..64 unique targets compatible with its context"
  ;;

  let context t = t.context
  let targets t = t.targets
end

module Suppression = struct
  type t =
    | Disabled
    | Scope_blocked
    | Native_unavailable
    | Composition
    | Text_input
    | Native_navigation
    | Conflict of Ui_command.Id.t
  [@@deriving equal, sexp_of]

  let of_wire = function
    | W.Suppression.Disabled -> Disabled
    | Scope_blocked -> Scope_blocked
    | Native_unavailable -> Native_unavailable
    | Composition -> Composition
    | Text_input -> Text_input
    | Native_navigation -> Native_navigation
    | Conflict id -> Conflict (Ui_command.Id.of_string id |> Or_error.ok_exn)
  ;;
end

module Disposition = struct
  type t =
    | Declared
    | Override
    | Native_first
    | Widget
    | Unavailable of Suppression.t
  [@@deriving equal, sexp_of]

  let of_wire = function
    | W.Disposition.Declared -> Declared
    | Override -> Override
    | Native_first -> Native_first
    | Widget -> Widget
    | Unavailable reason -> Unavailable (Suppression.of_wire reason)
  ;;
end

module Stroke = struct
  module Modifier = struct
    type t =
      | Control
      | Alt
      | Shift
      | Super
      | Function
    [@@deriving equal, sexp_of]
  end

  type t = W.Stroke.t [@@deriving equal, sexp_of]

  let key t = t.W.Stroke.key

  let modifiers t =
    List.filter_map
      [ 1L, Modifier.Control; 2L, Alt; 4L, Shift; 8L, Super; 16L, Function ]
      ~f:(fun (bit, modifier) ->
        Option.some_if Int64.(bit_and t.W.Stroke.modifiers bit <> 0L) modifier)
  ;;

  let key_label t ~platform ~spoken =
    match Shortcut.create ~key:t.W.Stroke.key () with
    | Ok shortcut ->
      if spoken
      then Shortcut.accessible_label shortcut ~platform
      else Shortcut.format shortcut ~platform
    | Error _ ->
      let decoded = Stdlib.String.get_utf_8_uchar t.key 0 in
      let first = Stdlib.Uchar.utf_decode_uchar decoded in
      let length = Stdlib.Uchar.utf_decode_length decoded in
      let buffer = Stdlib.Buffer.create (String.length t.key) in
      (match Uucp.Case.Map.to_upper first with
       | `Self -> Stdlib.Buffer.add_utf_8_uchar buffer first
       | `Uchars chars -> List.iter chars ~f:(Stdlib.Buffer.add_utf_8_uchar buffer));
      Stdlib.Buffer.add_string buffer (String.drop_prefix t.key length);
      Stdlib.Buffer.contents buffer
  ;;

  let label t ~platform ~spoken =
    let macos = Shortcut.Platform.equal platform Macos in
    let mods = modifiers t in
    let has modifier = List.mem mods modifier ~equal:Modifier.equal in
    let symbol glyph name = if macos && not spoken then glyph else name in
    let parts =
      List.filter_opt
        [ Option.some_if
            (has Function)
            (if spoken then "Function" else if macos then "fn" else "Fn")
        ; Option.some_if (has Control) (symbol "⌃" (if spoken then "Control" else "Ctrl"))
        ; Option.some_if
            (has Alt)
            (symbol "⌥" (if macos && spoken then "Option" else "Alt"))
        ; Option.some_if (has Shift) (symbol "⇧" "Shift")
        ; Option.some_if (has Super) (symbol "⌘" (if macos then "Command" else "Super"))
        ]
    in
    String.concat
      ~sep:(if spoken then " + " else if macos then "" else "+")
      (parts @ [ key_label t ~platform ~spoken ])
  ;;

  let format t ~platform = label t ~platform ~spoken:false
  let accessible_label t ~platform = label t ~platform ~spoken:true
end

let shortcut_of_wire (t : C.Shortcut.t) =
  Shortcut.create
    ~key:t.key
    ~modifiers:
      (List.map t.modifiers ~f:(function
         | C.Shortcut_modifier.Primary -> Shortcut.Modifier.Primary
         | Control -> Control
         | Alt -> Alt
         | Shift -> Shift
         | Super -> Super))
    ~priority:
      (match t.priority with
       | Native_first -> Native_first
       | Override -> Override)
    ~text_input:
      (match t.text_input with
       | Modified_only -> Modified_only
       | Always -> Always
       | Never -> Never)
    ~during_composition:t.during_composition
    ()
  |> Or_error.ok_exn
;;

module Candidate = struct
  type t =
    { shortcut : Shortcut.t
    ; disposition : Disposition.t
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : W.Candidate.t) =
    { shortcut = shortcut_of_wire t.shortcut
    ; disposition = Disposition.of_wire t.disposition
    }
  ;;
end

module Unsupported = struct
  type t =
    | Sequence_too_long
    | Invalid_stroke
  [@@deriving equal, sexp_of]
end

module Entry = struct
  type t =
    | Missing_command
    | Registry of
        { enabled : bool
        ; candidates : Candidate.t list
        }
    | Native_unbound
    | Native_binding of
        { strokes : Stroke.t list
        ; disposition : Disposition.t
        }
    | Native_unsupported of Unsupported.t
  [@@deriving equal, sexp_of]

  let of_wire = function
    | W.Entry.Missing_command -> Missing_command
    | Registry { enabled; candidates } ->
      Registry { enabled; candidates = List.map candidates ~f:Candidate.of_wire }
    | Native_unbound -> Native_unbound
    | Native_binding { strokes; disposition } ->
      Native_binding { strokes; disposition = Disposition.of_wire disposition }
    | Native_unsupported reason ->
      Native_unsupported
        (match reason with
         | Sequence_too_long -> Unsupported.Sequence_too_long
         | Invalid_stroke -> Invalid_stroke)
  ;;
end

module State = struct
  type t =
    | Ready of (Target.t * Entry.t) list
    | Suspended
    | Context_gone
    | Invalid_context
    | Epoch_exhausted
    | Capacity
  [@@deriving equal, sexp_of]
end

module Observation = struct
  type t =
    { epoch : int64
    ; state : State.t
    }
  [@@deriving equal, sexp_of]

  let epoch t = t.epoch
  let state t = t.state
end

module Expert = struct
  let to_wire = Config.to_wire

  let valid_window config window =
    match config.Config.context with
    | W.Context.Editor (target, _) -> Gpuio_protocol.Window_id.equal window target
    | Focused | Here | Native_context _ -> true
  ;;

  let of_wire config (observation : W.Observation.t) =
    if not (W.Observation.valid_for observation (to_wire config))
    then None
    else (
      let state =
        match observation.state with
        | Ready entries ->
          State.Ready
            (List.map2_exn config.Config.targets entries ~f:(fun target entry ->
               target, Entry.of_wire entry))
        | Suspended -> Suspended
        | Context_gone -> Context_gone
        | Invalid_context -> Invalid_context
        | Epoch_exhausted -> Epoch_exhausted
        | Capacity -> Capacity
      in
      Some { Observation.epoch = observation.epoch; state })
  ;;
end
