open Core
module Wire = Gpuio_protocol.Document_diff_wire

module File_key = struct
  type t =
    | Path of string
    | Unnamed
  [@@deriving equal, compare, sexp_of]

  let to_wire = function
    | Path path -> Wire.File_key.Path path
    | Unnamed -> Unnamed
  ;;

  let of_wire = function
    | Wire.File_key.Path path -> Path path
    | Unnamed -> Unnamed
  ;;
end

module Collapse = struct
  type t =
    | Managed of { initially_collapsed : File_key.t list }
    | Controlled of File_key.t list
  [@@deriving equal, sexp_of]

  let to_wire = function
    | Managed { initially_collapsed } ->
      Wire.Collapse.Managed (List.map initially_collapsed ~f:File_key.to_wire)
    | Controlled keys -> Controlled (List.map keys ~f:File_key.to_wire)
  ;;

  let of_wire = function
    | Wire.Collapse.Managed keys ->
      Managed { initially_collapsed = List.map keys ~f:File_key.of_wire }
    | Controlled keys -> Controlled (List.map keys ~f:File_key.of_wire)
  ;;
end

module Line_limit = struct
  type t =
    | Managed of
        { initial : int option
        ; step : int
        }
    | Controlled of int option
  [@@deriving equal, sexp_of]

  let to_wire = function
    | Managed { initial; step } ->
      Wire.Line_limit.Managed
        { initial = Option.map initial ~f:Int64.of_int; step = Int64.of_int step }
    | Controlled limit -> Controlled (Option.map limit ~f:Int64.of_int)
  ;;

  let of_wire = function
    | Wire.Line_limit.Managed { initial; step } ->
      Managed
        { initial = Option.map initial ~f:Int64.to_int_exn; step = Int64.to_int_exn step }
    | Controlled limit -> Controlled (Option.map limit ~f:Int64.to_int_exn)
  ;;
end

module Config = struct
  type t = Wire.Config.t [@@deriving equal, sexp_of]

  let create
        ?(collapse = Collapse.Managed { initially_collapsed = [] })
        ?(line_limit = Line_limit.Managed { initial = None; step = 200 })
        ?(word_diff = true)
        ()
    =
    let t : Wire.Config.t =
      { collapse = Collapse.to_wire collapse
      ; line_limit = Line_limit.to_wire line_limit
      ; word_diff
      }
    in
    if Wire.Config.valid t
    then Ok t
    else Or_error.error_string "invalid diff keys, line limit, step or configuration size"
  ;;

  let default = create () |> Or_error.ok_exn
  let collapse t = Collapse.of_wire t.Wire.Config.collapse
  let line_limit t = Line_limit.of_wire t.Wire.Config.line_limit
  let word_diff t = t.Wire.Config.word_diff
end

module File = struct
  type t =
    { index : int
    ; key : File_key.t
    ; before_path : string option
    ; after_path : string option
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.File.t) =
    { index = Int64.to_int_exn t.index
    ; key = File_key.of_wire t.key
    ; before_path = t.before_path
    ; after_path = t.after_path
    }
  ;;
end

module Line = struct
  type t =
    { file : File.t
    ; before : int option
    ; after : int option
    ; start_byte : int
    ; end_byte : int
    ; text : string
    }
  [@@deriving equal, sexp_of]

  let of_wire (t : Wire.Line.t) =
    { file = File.of_wire t.file
    ; before = Option.map t.before ~f:Int64.to_int_exn
    ; after = Option.map t.after ~f:Int64.to_int_exn
    ; start_byte = Int64.to_int_exn t.start_byte
    ; end_byte = Int64.to_int_exn t.end_byte
    ; text = t.text
    }
  ;;
end

module Observation = struct
  type t =
    | Toggle_file of
        { file : File.t
        ; collapsed : bool
        ; applied : bool
        }
    | Show_more of
        { visible : int
        ; hidden : int
        ; applied_limit : int option
        }
    | Line of Line.t
  [@@deriving equal, sexp_of]

  let of_wire = function
    | Wire.Observation.Toggle_file { file; collapsed; applied } ->
      Toggle_file { file = File.of_wire file; collapsed; applied }
    | Show_more { visible; hidden; applied_limit } ->
      Show_more
        { visible = Int64.to_int_exn visible
        ; hidden = Int64.to_int_exn hidden
        ; applied_limit = Option.map applied_limit ~f:Int64.to_int_exn
        }
    | Line line -> Line (Line.of_wire line)
  ;;
end

module Event = struct
  type t =
    { source_revision : int64
    ; source_generation : int64
    ; observation : Observation.t
    }
  [@@deriving equal, sexp_of]
end

module Expert = struct
  let to_wire t = t

  let of_wire t =
    if Wire.Config.valid t
    then Ok t
    else Or_error.error_string "invalid diff configuration"
  ;;

  let event_of_wire ~config (event : Wire.Event.t) =
    if not (Wire.Event.valid event && Wire.Observation.valid_for event.observation config)
    then Or_error.error_string "invalid diff event or control ownership"
    else
      Ok
        { Event.source_revision = event.source_revision
        ; source_generation = event.source_generation
        ; observation = Observation.of_wire event.observation
        }
  ;;
end
