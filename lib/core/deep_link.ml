open Core

let ascii_letter = function
  | 'a' .. 'z' | 'A' .. 'Z' -> true
  | _ -> false
;;

let digit = function
  | '0' .. '9' -> true
  | _ -> false
;;

module Scheme = struct
  type t = string [@@deriving equal, compare, sexp_of]

  let of_string value =
    let valid_char c =
      ascii_letter c
      || digit c
      ||
      match c with
      | '+' | '-' | '.' -> true
      | _ -> false
    in
    if
      String.is_empty value
      || String.length value > 64
      || (not (ascii_letter value.[0]))
      || not (String.for_all value ~f:valid_char)
    then Or_error.error_string "scheme must be 1..64 ASCII scheme characters"
    else Ok (String.lowercase value)
  ;;

  let to_string t = t
end

module Error = struct
  type t =
    | Too_long
    | Invalid_scheme
    | Unsupported_scheme
    | Invalid_authority
    | Invalid_path
    | Invalid_query
    | Invalid_fragment
  [@@deriving equal, sexp_of]
end

type t =
  { original : string
  ; scheme : Scheme.t
  ; route : string
  ; path : string
  ; query : string option
  ; fragment : string option
  }
[@@deriving equal, sexp_of]

let max_bytes = 16_384

let unreserved c =
  ascii_letter c
  || digit c
  ||
  match c with
  | '-' | '.' | '_' | '~' -> true
  | _ -> false
;;

let pchar c =
  unreserved c
  ||
  match c with
  | '!' | '$' | '&' | '\'' | '(' | ')' | '*' | '+' | ',' | ';' | '=' | ':' | '@' -> true
  | _ -> false
;;

let hex = function
  | '0' .. '9' | 'a' .. 'f' | 'A' .. 'F' -> true
  | _ -> false
;;

let valid_component text ~query =
  let length = String.length text in
  let rec loop i =
    if i = length
    then true
    else if Char.equal text.[i] '%'
    then i + 2 < length && hex text.[i + 1] && hex text.[i + 2] && loop (i + 3)
    else
      (pchar text.[i] || Char.equal text.[i] '/' || (query && Char.equal text.[i] '?'))
      && loop (i + 1)
  in
  loop 0
;;

let split_optional text ~on =
  match String.lsplit2 text ~on with
  | None -> text, None
  | Some (prefix, suffix) -> prefix, Some suffix
;;

let of_string ~schemes original =
  let open Result.Let_syntax in
  if String.length original > max_bytes
  then Error Error.Too_long
  else (
    let%bind raw_scheme, rest =
      Result.of_option (String.lsplit2 original ~on:':') ~error:Error.Invalid_scheme
    in
    let%bind scheme =
      Result.map_error (Scheme.of_string raw_scheme) ~f:(fun _ -> Error.Invalid_scheme)
    in
    if not (List.mem schemes scheme ~equal:Scheme.equal)
    then Error Error.Unsupported_scheme
    else (
      let%bind rest =
        Result.of_option
          (String.chop_prefix rest ~prefix:"//")
          ~error:Error.Invalid_authority
      in
      let rest, fragment = split_optional rest ~on:'#' in
      let rest, query = split_optional rest ~on:'?' in
      let route, path =
        match String.lsplit2 rest ~on:'/' with
        | None -> rest, ""
        | Some (route, suffix) -> route, "/" ^ suffix
      in
      if String.is_empty route || not (String.for_all route ~f:unreserved)
      then Error Error.Invalid_authority
      else if not (valid_component path ~query:false)
      then Error Error.Invalid_path
      else if not (Option.for_all query ~f:(valid_component ~query:true))
      then Error Error.Invalid_query
      else if not (Option.for_all fragment ~f:(valid_component ~query:true))
      then Error Error.Invalid_fragment
      else Ok { original; scheme; route; path; query; fragment }))
;;

let scheme t = t.scheme
let route t = t.route
let path t = t.path
let query t = t.query
let fragment t = t.fragment
let to_string t = t.original
