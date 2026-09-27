open Core

(** Incoming application links. Parsing is pure: it never opens a resource,
    executes an action or resolves a window. This deliberately supports a
    hierarchical application-link profile, not arbitrary browser URLs. *)
module Scheme : sig
  type t [@@deriving equal, compare, sexp_of]

  (** ASCII letter followed by letters, digits, [+], [-] or [.], at most 64
      bytes. Stored lowercase. Construction does not register an OS handler. *)
  val of_string : string -> t Or_error.t

  val to_string : t -> string
end

module Error : sig
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

type t [@@deriving equal, sexp_of]

val max_bytes : int

(** Parse [scheme://route/path?query#fragment], at most 16 KiB, allowing only
    [schemes]. Route is a nonempty ASCII unreserved identifier (letters, digits,
    [-._~]); credentials, ports and network-address syntax are not accepted.
    Path/query/fragment use RFC 3986 character and percent-escape syntax.
    Unicode must be percent encoded. Empty paths and empty query/fragment are
    distinct from absent query/fragment. No decoding, dot-segment removal,
    case-folding of the route, or query reordering occurs.

    Application routing must validate its own routes, parameters and any decoded
    values. A valid envelope grants no filesystem/network authority. *)
val of_string : schemes:Scheme.t list -> string -> (t, Error.t) Result.t

val scheme : t -> Scheme.t
val route : t -> string
val path : t -> string
val query : t -> string option
val fragment : t -> string option

(** Original bytes, including original scheme spelling. *)
val to_string : t -> string
