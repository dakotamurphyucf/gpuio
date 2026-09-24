open Core

exception Invalid_wire_data

let read_bytes buffer ~pos_ref ~maximum ~valid =
  let start = !pos_ref in
  let length = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
  if length > maximum then raise Invalid_wire_data;
  if length > Bigstring.length buffer - !pos_ref then raise Bin_prot.Common.Buffer_short;
  pos_ref := start;
  let value = Bin_prot.Read.bin_read_string buffer ~pos_ref in
  if not (valid value) then raise Invalid_wire_data;
  value
;;

module Bounded_string (Spec : sig
    val maximum : int
    val valid : string -> bool
  end) =
struct
  type t = string [@@deriving bin_io, equal, compare, sexp_of]

  let bin_read_t buffer ~pos_ref =
    read_bytes buffer ~pos_ref ~maximum:Spec.maximum ~valid:Spec.valid
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

let valid_text value =
  Stdlib.String.is_valid_utf_8 value && not (String.contains value '\000')
;;

module Bytes = Bounded_string (struct
    let maximum = 262_144
    let valid _ = true
  end)

module Text = Bounded_string (struct
    let maximum = 262_144
    let valid = valid_text
  end)

module Label = Bounded_string (struct
    let maximum = 4096
    let valid value = valid_text value && not (String.is_empty (String.strip value))
  end)

module Kind = Bounded_string (struct
    let maximum = 128

    let valid value =
      (not (String.is_empty value))
      && String.for_all value ~f:(function
        | 'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '.' | '_' | '-' | '/' | '+' -> true
        | _ -> false)
    ;;
  end)

module Format = struct
  type t =
    | Text
    | Files
    | Custom of Kind.t
  [@@deriving bin_io, equal, compare, sexp_of]
end

module File = struct
  type t =
    { path : string
    ; is_directory : bool option
    }
  [@@deriving bin_io, equal, sexp_of]

  let read buffer ~pos_ref ~remaining_bytes =
    let path =
      read_bytes
        buffer
        ~pos_ref
        ~maximum:(Int.min 16_384 remaining_bytes)
        ~valid:(fun path ->
          (not (String.is_empty path))
          && Char.equal path.[0] '/'
          && not (String.contains path '\000'))
    in
    let is_directory = [%bin_read: bool option] buffer ~pos_ref in
    { path; is_directory }
  ;;

  let bin_read_t buffer ~pos_ref = read buffer ~pos_ref ~remaining_bytes:16_384
  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module Files = struct
  type t = File.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t buffer ~pos_ref =
    let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
    if count <= 0 || count > 128 then raise Invalid_wire_data;
    let rec read remaining remaining_bytes reversed =
      if remaining = 0
      then List.rev reversed
      else (
        let file = File.read buffer ~pos_ref ~remaining_bytes in
        read (remaining - 1) (remaining_bytes - String.length file.path) (file :: reversed))
    in
    read count 262_144 []
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module Payload = struct
  type t =
    | Text of Text.t
    | Files of Files.t
    | Custom of
        { kind : Kind.t
        ; data : Bytes.t
        }
  [@@deriving bin_io, equal, sexp_of]
end

module Source = struct
  type t =
    { label : Label.t
    ; payload : Payload.t
    ; disabled : bool
    ; allow_desktop_files : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let bin_read_t buffer ~pos_ref =
    let t = bin_read_t buffer ~pos_ref in
    if t.allow_desktop_files
    then (
      match t.payload with
      | Files files ->
        if not (List.for_all files ~f:(fun f -> Option.is_some f.is_directory))
        then raise Invalid_wire_data
      | Text _ | Custom _ -> raise Invalid_wire_data);
    t
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module Formats = struct
  type t = Format.t list [@@deriving bin_io, equal, sexp_of]

  let bin_read_t buffer ~pos_ref =
    let count = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
    if count <= 0 || count > 16 then raise Invalid_wire_data;
    (* Read in wire order explicitly; List.init's callback evaluation order is
       not a sequencing contract for this stateful cursor. *)
    let rec read remaining reversed =
      if remaining = 0
      then List.rev reversed
      else (
        let format = Format.bin_read_t buffer ~pos_ref in
        read (remaining - 1) (format :: reversed))
    in
    let values = read count [] in
    if List.contains_dup values ~compare:Format.compare then raise Invalid_wire_data;
    values
  ;;

  let bin_reader_t = { bin_reader_t with read = bin_read_t }
  let bin_t = { bin_t with reader = bin_reader_t }
end

module Target = struct
  type t =
    { label : Label.t
    ; accepted_formats : Formats.t
    ; disabled : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Origin = struct
  type t =
    | Internal
    | Desktop
  [@@deriving bin_io, equal, sexp_of]
end

module Offer = struct
  type t =
    { format : Format.t
    ; data_bytes : int64
    ; file_count : int64
    ; origin : Origin.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let is_valid t =
    Int64.(t.data_bytes >= 0L && t.data_bytes <= 262_144L)
    &&
    match t.format with
    | Files ->
      Int64.(t.file_count >= 1L && t.file_count <= 128L && t.data_bytes >= t.file_count)
    | Text | Custom _ -> Int64.equal t.file_count 0L
  ;;
end

module Cancel_reason = struct
  type t =
    | Escape
    | Hidden
    | Blocked
    | Disabled
    | Removed
    | Reconfigured
    | Window_closed
    | Window_inactive
  [@@deriving bin_io, equal, sexp_of]
end

module Outcome = struct
  type t =
    | Internal_drop
    | Cancelled of Cancel_reason.t
    | Unconfirmed
  [@@deriving bin_io, equal, sexp_of]
end

module Source_phase = struct
  type t =
    | Started of Payload.t
    | Desktop_offered
    | Desktop_unavailable
    | Ended of Outcome.t
  [@@deriving bin_io, equal, sexp_of]
end

module Source_sample = struct
  type t =
    { gesture : int64
    ; phase : Source_phase.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let is_valid t = Int64.(t.gesture > 0L)
end

module Rejection = struct
  type t =
    | Invalid_data
    | Limit_exceeded
  [@@deriving bin_io, equal, sexp_of]
end

module Target_phase = struct
  type t =
    | Entered of Offer.t
    | Moved
    | Left
    | Dropped of Payload.t
    | Rejected of Rejection.t
  [@@deriving bin_io, equal, sexp_of]
end

module Modifiers = struct
  type t =
    { shift : bool
    ; control : bool
    ; alt : bool
    ; command : bool
    ; function_ : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Target_sample = struct
  type t =
    { gesture : int64
    ; phase : Target_phase.t
    ; window_x : float
    ; window_y : float
    ; local_x : float
    ; local_y : float
    ; modifiers : Modifiers.t
    }
  [@@deriving bin_io, equal, sexp_of]

  let is_valid t =
    Int64.(t.gesture > 0L)
    && List.for_all [ t.window_x; t.window_y; t.local_x; t.local_y ] ~f:Float.is_finite
    &&
    match t.phase with
    | Entered offer -> Offer.is_valid offer
    | Moved | Left | Dropped _ | Rejected _ -> true
  ;;
end
