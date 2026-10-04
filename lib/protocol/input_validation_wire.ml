open Core

let max_source_bytes = 2048
let max_diagnostic_bytes = 1024

module Matching = struct
  type t =
    | Whole_value
    | Substring
  [@@deriving bin_io, equal, sexp_of]
end

module Source = struct
  type t =
    { pattern : string
    ; matching : Matching.t
    ; case_sensitive : bool
    }
  [@@deriving bin_io, equal, sexp_of]

  let is_valid t =
    String.length t.pattern <= max_source_bytes
    && Stdlib.String.is_valid_utf_8 t.pattern
    && not (String.contains t.pattern '\000')
  ;;

  let encode t =
    if is_valid t
    then Ok (Bin_prot.Utils.bin_dump bin_writer_t t |> Bigstring.to_string)
    else Or_error.error_string "invalid regex source bounds or UTF-8"
  ;;
end

module Rule = struct
  type t =
    { regex : Source.t
    ; allow_empty : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Invalid_source
    | Invalid_regex of string
    | Too_complex
  [@@deriving bin_io, equal, sexp_of]
end

module Preparation = struct
  type t =
    | Checked
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let decode bytes =
    if String.length bytes > max_diagnostic_bytes + 16
    then Or_error.error_string "oversized regex preparation result"
    else
      Or_error.try_with (fun () ->
        let pos_ref = ref 0 in
        let buffer = Bigstring.of_string bytes in
        let tag () = Bin_prot.Read.bin_read_int_8bit buffer ~pos_ref in
        let result =
          match tag () with
          | 0 -> Checked
          | 1 ->
            (match tag () with
             | 0 -> Failed Invalid_source
             | 2 -> Failed Too_complex
             | 1 ->
               let length = (Bin_prot.Read.bin_read_nat0 buffer ~pos_ref :> int) in
               if
                 length > max_diagnostic_bytes
                 || length < 0
                 || length <> String.length bytes - !pos_ref
               then failwith "invalid regex diagnostic length";
               let message = String.sub bytes ~pos:!pos_ref ~len:length in
               pos_ref := !pos_ref + length;
               if
                 (not (Stdlib.String.is_valid_utf_8 message))
                 || String.contains message '\000'
               then failwith "invalid regex diagnostic";
               Failed (Invalid_regex message)
             | _ -> failwith "unknown regex preparation error")
          | _ -> failwith "unknown regex preparation result"
        in
        if !pos_ref <> String.length bytes
        then failwith "trailing regex preparation bytes";
        result)
  ;;
end
