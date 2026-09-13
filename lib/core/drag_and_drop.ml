open Core

module Custom_kind = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string value =
    if
      String.is_empty value
      || String.length value > 128
      || not
           (String.for_all value ~f:(function
              | 'a' .. 'z' | 'A' .. 'Z' | '0' .. '9' | '.' | '_' | '-' | '/' | '+' -> true
              | _ -> false))
    then Or_error.error_string "custom drag kind must be 1..128 ASCII identifier bytes"
    else Ok value
  ;;

  let to_string t = t
end

module Format = struct
  type t =
    | Text
    | Files
    | Custom of Custom_kind.t
  [@@deriving equal, compare, sexp_of]
end

module File = struct
  type t =
    { path : File_path.t
    ; is_directory : bool option
    }
  [@@deriving equal, sexp_of]

  let create ?is_directory path = { path; is_directory }
end

module Payload = struct
  type t =
    | Text of string
    | Files of File.t list
    | Custom of
        { kind : Custom_kind.t
        ; data : string
        }
  [@@deriving equal, sexp_of]

  let max_bytes = 262_144
  let max_files = 128

  let text text =
    if
      String.length text > max_bytes
      || String.contains text '\000'
      || not (Stdlib.String.is_valid_utf_8 text)
    then Or_error.error_string "drag text must be UTF-8 without NUL, at most 262144 bytes"
    else Ok (Text text)
  ;;

  let files files =
    (* Stop at either limit, including for caller-owned oversized lists. *)
    let rec within_limits remaining total = function
      | [] -> true
      | { File.path; is_directory = _ } :: rest ->
        let total = total + String.length (File_path.to_string path) in
        remaining > 0 && total <= max_bytes && within_limits (remaining - 1) total rest
    in
    if List.is_empty files || not (within_limits max_files 0 files)
    then
      Or_error.error_string "drag files require 1..128 paths, at most 262144 path bytes"
    else Ok (Files files)
  ;;

  let custom ~kind ~data =
    if String.length data > max_bytes
    then Or_error.error_string "custom drag data exceeds 262144 bytes"
    else Ok (Custom { kind; data })
  ;;

  let format = function
    | Text _ -> Format.Text
    | Files _ -> Format.Files
    | Custom { kind; data = _ } -> Format.Custom kind
  ;;

  let data_bytes = function
    | Text text -> String.length text
    | Custom { kind = _; data } -> String.length data
    | Files files ->
      List.sum
        (module Int)
        files
        ~f:(fun file -> String.length (File_path.to_string file.path))
  ;;
end

let validate_label label =
  if
    String.length label > 4096
    || String.is_empty (String.strip label)
    || String.contains label '\000'
    || not (Stdlib.String.is_valid_utf_8 label)
  then Or_error.error_string "drag label must be bounded nonblank UTF-8 without NUL"
  else Ok ()
;;

module Source = struct
  type t =
    { label : string
    ; payload : Payload.t
    ; disabled : bool
    ; allow_desktop_files : bool
    }
  [@@deriving equal, sexp_of]

  let create ~label ~payload ?(disabled = false) ?(allow_desktop_files = false) () =
    let open Or_error.Let_syntax in
    let%bind () = validate_label label in
    let can_offer_files =
      match payload with
      | Payload.Files files ->
        List.for_all files ~f:(fun f -> Option.is_some f.is_directory)
      | Text _ | Custom _ -> false
    in
    if allow_desktop_files && not can_offer_files
    then
      Or_error.error_string
        "desktop drag offering requires files with known directory metadata"
    else Ok { label; payload; disabled; allow_desktop_files }
  ;;

  let label t = t.label
  let payload t = t.payload
  let disabled t = t.disabled
  let allow_desktop_files t = t.allow_desktop_files
end

module Target = struct
  type t =
    { label : string
    ; accepted_formats : Format.t list
    ; disabled : bool
    }
  [@@deriving equal, sexp_of]

  let create ~label ~accept ?(disabled = false) () =
    let open Or_error.Let_syntax in
    let%bind () = validate_label label in
    let bounded = List.length (List.take accept 17) in
    if bounded = 0 || bounded > 16 || List.contains_dup accept ~compare:Format.compare
    then Or_error.error_string "drop target requires 1..16 distinct formats"
    else Ok { label; accepted_formats = accept; disabled }
  ;;

  let label t = t.label
  let accepted_formats t = t.accepted_formats
  let disabled t = t.disabled

  let accepts t payload =
    (not t.disabled)
    && List.mem t.accepted_formats (Payload.format payload) ~equal:Format.equal
  ;;
end

module Expert = struct
  module Wire = Gpuio_protocol.Wire.Drag_and_drop

  let payload_to_wire = function
    | Payload.Text text -> Wire.Payload.Text text
    | Files files ->
      Files
        (List.map files ~f:(fun { File.path; is_directory } ->
           Wire.File.{ path = File_path.to_string path; is_directory }))
    | Custom { kind; data } -> Custom { kind = Custom_kind.to_string kind; data }
  ;;

  let payload_of_wire = function
    | Wire.Payload.Text text -> Payload.text text
    | Files files ->
      let open Or_error.Let_syntax in
      if List.length (List.take files 129) > 128
      then Or_error.error_string "drag files exceed 128 entries"
      else (
        let%bind files =
          List.map files ~f:(fun { Wire.File.path; is_directory } ->
            let%map path = File_path.of_string path in
            File.create ?is_directory path)
          |> Or_error.all
        in
        Payload.files files)
    | Custom { kind; data } ->
      let open Or_error.Let_syntax in
      let%bind kind = Custom_kind.of_string kind in
      Payload.custom ~kind ~data
  ;;

  let source_to_wire (t : Source.t) =
    Wire.Source.
      { label = t.label
      ; payload = payload_to_wire t.payload
      ; disabled = t.disabled
      ; allow_desktop_files = t.allow_desktop_files
      }
  ;;

  let target_to_wire (t : Target.t) =
    Wire.Target.
      { label = t.label
      ; accepted_formats =
          List.map t.accepted_formats ~f:(function
            | Format.Text -> Wire.Format.Text
            | Files -> Files
            | Custom kind -> Custom (Custom_kind.to_string kind))
      ; disabled = t.disabled
      }
  ;;
end
