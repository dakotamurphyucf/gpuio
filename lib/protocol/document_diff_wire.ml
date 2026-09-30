open Core

exception Invalid_wire_data

let max_keys = 8192
let max_config_bytes = 262144
let max_event_bytes = 32768

module Bounded_string (Spec : sig
    val maximum : int
    val valid : string -> bool
  end) =
struct
  type t = string [@@deriving bin_io, equal, compare, sexp_of]

  let bin_read_t buffer ~pos_ref =
    let start = !pos_ref in
    let length = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
    if length > Spec.maximum then raise Invalid_wire_data;
    if length > Bigstring.length buffer - !pos_ref then raise Bin_prot.Common.Buffer_short;
    pos_ref := start;
    let text = Bin_prot.Read.bin_read_string buffer ~pos_ref in
    if not (Spec.valid text) then raise Invalid_wire_data;
    text
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

let valid_path text =
  (not (String.is_empty text))
  && String.length text <= 4096
  && Stdlib.String.is_valid_utf_8 text
  && not (String.contains text '\000')
;;

module Path = Bounded_string (struct
    let maximum = 4096
    let valid = valid_path
  end)

module Line_text = Bounded_string (struct
    let maximum = 16384
    let valid text = Stdlib.String.is_valid_utf_8 text && not (String.contains text '\n')
  end)

module File_key = struct
  type t =
    | Path of Path.t
    | Unnamed
  [@@deriving bin_io, equal, compare, sexp_of]

  let valid = function
    | Path path -> valid_path path
    | Unnamed -> true
  ;;
end

module Keys = struct
  type t = File_key.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t buffer ~pos_ref =
    let start = !pos_ref in
    let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
    if count > max_keys then raise Invalid_wire_data;
    if count > Bigstring.length buffer - !pos_ref then raise Bin_prot.Common.Buffer_short;
    let rec read remaining reversed =
      if !pos_ref - start > max_config_bytes then raise Invalid_wire_data;
      if remaining = 0
      then List.rev reversed
      else (
        let key = File_key.bin_read_t buffer ~pos_ref in
        read (remaining - 1) (key :: reversed))
    in
    read count []
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module Collapse = struct
  type t =
    | Managed of Keys.t
    | Controlled of Keys.t
  [@@deriving bin_io, equal, sexp_of]

  let keys = function
    | Managed keys | Controlled keys -> keys
  ;;
end

module Line_limit = struct
  type t =
    | Managed of
        { initial : int64 option
        ; step : int64
        }
    | Controlled of int64 option
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let limit = Option.for_all ~f:(fun n -> Int64.(n >= 0L && n <= 8192L)) in
    match t with
    | Managed { initial; step } -> limit initial && Int64.(step >= 1L && step <= 8192L)
    | Controlled value -> limit value
  ;;
end

module Config = struct
  type t =
    { collapse : Collapse.t
    ; line_limit : Line_limit.t
    ; word_diff : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let keys = Collapse.keys t.collapse in
    List.length keys <= max_keys
    && List.for_all keys ~f:File_key.valid
    && (not (List.contains_dup keys ~compare:File_key.compare))
    && Line_limit.valid t.line_limit
    && bin_size_t t <= max_config_bytes
  ;;

  let bin_read_t buffer ~pos_ref =
    let start = !pos_ref in
    let t = bin_read_t buffer ~pos_ref in
    if !pos_ref - start > max_config_bytes || not (valid t) then raise Invalid_wire_data;
    t
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module File = struct
  type t =
    { index : int64
    ; key : File_key.t
    ; before_path : Path.t option
    ; after_path : Path.t option
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.index >= 0L && t.index < 8192L)
    && Option.for_all t.before_path ~f:valid_path
    && Option.for_all t.after_path ~f:valid_path
    &&
    match t.key, Option.first_some t.after_path t.before_path with
    | Path key, Some path -> String.equal key path
    | Unnamed, None -> true
    | Path _, None | Unnamed, Some _ -> false
  ;;
end

module Line = struct
  type t =
    { file : File.t
    ; before : int64 option
    ; after : int64 option
    ; start_byte : int64
    ; end_byte : int64
    ; text : Line_text.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    let coordinate n = Int64.(n > 0L && n <= 2147483647L) in
    File.valid t.file
    && Option.for_all t.before ~f:coordinate
    && Option.for_all t.after ~f:coordinate
    && Int64.(t.start_byte >= 0L && t.end_byte >= t.start_byte && t.end_byte <= 262144L)
    && Int64.equal Int64.(t.end_byte - t.start_byte) (Int64.of_int (String.length t.text))
    && String.length t.text <= 16384
    && Stdlib.String.is_valid_utf_8 t.text
    && not (String.contains t.text '\n')
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
        { visible : int64
        ; hidden : int64
        ; applied_limit : int64 option
        }
    | Line of Line.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Toggle_file { file; collapsed = _; applied = _ } -> File.valid file
    | Show_more { visible; hidden; applied_limit } ->
      Int64.(
        visible >= 0L
        && visible < 8192L
        && hidden > 0L
        && hidden <= 8192L
        && visible + hidden <= 8192L)
      && Option.for_all applied_limit ~f:(fun n -> Int64.(n > visible && n <= 8192L))
    | Line line -> Line.valid line
  ;;

  let valid_for t (config : Config.t) =
    valid t
    && Config.valid config
    &&
    match t with
    | Toggle_file { file; collapsed; applied } ->
      (match config.collapse with
       | Managed _ -> applied
       | Controlled keys ->
         (not applied)
         && not (Bool.equal collapsed (List.mem keys file.key ~equal:File_key.equal)))
    | Show_more { visible; hidden = _; applied_limit } ->
      (match config.line_limit with
       | Managed { initial = _; step } ->
         Option.equal
           Int64.equal
           applied_limit
           (Some (Int64.min 8192L Int64.(visible + step)))
       | Controlled limit ->
         Option.equal Int64.equal limit (Some visible) && Option.is_none applied_limit)
    | Line _ -> true
  ;;
end

module Event = struct
  type t =
    { config_epoch : int64
    ; source_revision : int64
    ; source_generation : int64
    ; observation : Observation.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.config_epoch > 0L && t.source_revision > 0L && t.source_generation > 0L)
    && Observation.valid t.observation
  ;;

  let bin_read_t buffer ~pos_ref =
    let start = !pos_ref in
    let t = bin_read_t buffer ~pos_ref in
    if !pos_ref - start > max_event_bytes || not (valid t) then raise Invalid_wire_data;
    t
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end
