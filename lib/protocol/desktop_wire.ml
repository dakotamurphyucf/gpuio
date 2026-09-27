open Core

let max_identifier_bytes = 128
let max_name_bytes = 256
let max_scheme_bytes = 64
let max_schemes = 16
let max_link_bytes = 16_384
let max_links = 64
let max_link_batch_bytes = 262_144

let letter = function
  | 'a' .. 'z' -> true
  | _ -> false
;;

let digit = function
  | '0' .. '9' -> true
  | _ -> false
;;

let valid_scheme s =
  let char c =
    letter c || digit c || Char.equal c '+' || Char.equal c '-' || Char.equal c '.'
  in
  (not (String.is_empty s))
  && String.length s <= max_scheme_bytes
  && letter s.[0]
  && String.for_all s ~f:char
;;

let valid_identifier s =
  let segments = String.split s ~on:'.' in
  String.length s <= max_identifier_bytes
  && List.length segments >= 2
  && List.for_all segments ~f:(fun part ->
    (not (String.is_empty part))
    && letter part.[0]
    && (not (Char.equal part.[String.length part - 1] '-'))
    && String.for_all part ~f:(fun c -> letter c || digit c || Char.equal c '-'))
;;

let valid_name s =
  (not (String.is_empty (String.strip s)))
  && String.length s <= max_name_bytes
  && Stdlib.String.is_valid_utf_8 s
  && String.for_all s ~f:(fun c -> Char.to_int c >= 32 && Char.to_int c <> 127)
;;

module Identity = struct
  type t =
    { identifier : string
    ; name : string
    ; schemes : string list
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    valid_identifier t.identifier
    && valid_name t.name
    && List.length t.schemes <= max_schemes
    && List.for_all t.schemes ~f:valid_scheme
    && not (List.contains_dup t.schemes ~compare:String.compare)
  ;;
end

module Capabilities = struct
  type t =
    { incoming_links : bool
    ; runtime_registration : bool
    ; application_activation : bool
    ; file_reveal : bool
    ; file_open : bool
    ; document_metadata : bool
    }
  [@@deriving bin_io, equal, sexp_of]
end

module Error = struct
  type t =
    | Invalid_request
    | Not_ready
    | Already_configured
    | Unsupported
    | Unavailable
    | Denied
    | Busy
    | Closed
    | Native_failure
  [@@deriving bin_io, equal, sexp_of]
end

module Request = struct
  type t =
    | Configure of Identity.t
    | Capabilities
    | Take_links
    | Activate of bool
    | Reveal_file of string
    | Open_file of string
    | Register_scheme of string
  [@@deriving bin_io, equal, sexp_of]

  let valid_path path =
    (not (String.is_empty path))
    && String.length path <= 16_384
    && Char.equal path.[0] '/'
    && not (String.contains path '\000')
  ;;

  let valid = function
    | Configure identity -> Identity.valid identity
    | Reveal_file path | Open_file path -> valid_path path
    | Register_scheme scheme -> valid_scheme scheme
    | Capabilities | Take_links | Activate _ -> true
  ;;
end

module Link_batch = struct
  type t =
    { links : string list
    ; dropped : int64
    }
  [@@deriving bin_io, equal, sexp_of]

  let valid t =
    Int64.(t.dropped >= 0L)
    && List.length t.links <= max_links
    && List.for_all t.links ~f:(fun link -> String.length link <= max_link_bytes)
    && List.sum (module Int) t.links ~f:String.length <= max_link_batch_bytes
  ;;
end

module Response = struct
  type t =
    | Configured
    | Capabilities of Capabilities.t
    | Links of Link_batch.t
    | Requested
    | Registered
    | Failed of Error.t
  [@@deriving bin_io, equal, sexp_of]

  let valid = function
    | Links batch -> Link_batch.valid batch
    | Configured | Capabilities _ | Requested | Registered | Failed _ -> true
  ;;
end
